// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Anonymous visit counting. Only totals reach the disk; the daily salt and
//! the visitor hashes live in memory and are dropped at 00:00 UTC.

use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::clock;
use crate::ua;

const COUNTERS: TableDefinition<&str, u64> = TableDefinition::new("counters");

/// Pages a view may name; anything else is dropped.
pub const PAGES: [&str; 9] = [
    "editor",
    "help",
    "desktop",
    "privacy",
    "terms",
    "disclaimer",
    "storage",
    "license",
    "contacts",
];
/// Kinds of errors the editor reports.
pub const ERRORS: [&str; 10] = [
    "too_large",
    "empty",
    "not_base64",
    "bad_length",
    "bad_padding",
    "not_utf8",
    "not_json",
    "not_object",
    "io",
    "other",
];

/// One accepted event from the page.
pub struct Hit<'a> {
    pub event: &'a str,
    pub page: Option<&'static str>,
    pub referrer: Option<String>,
    pub detail: Option<&'static str>,
    pub lang: Option<&'static str>,
}

pub struct Stats {
    db: Database,
    day: i64,
    salt: [u8; 32],
    seen_day: HashSet<[u8; 16]>,
    seen_hour: HashSet<(u32, [u8; 16])>,
}

fn fresh_salt() -> [u8; 32] {
    let mut s = [0u8; 32];
    // Without randomness the counts would still work, only less private; refuse instead.
    getrandom::fill(&mut s).expect("system random source");
    s
}

impl Stats {
    pub fn open(dir: &Path) -> Result<Stats, String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let db = Database::create(dir.join("stats.redb")).map_err(|e| format!("stats db: {e}"))?;
        let w = db.begin_write().map_err(|e| e.to_string())?;
        w.open_table(COUNTERS).map_err(|e| e.to_string())?;
        w.commit().map_err(|e| e.to_string())?;
        Ok(Stats {
            db,
            day: clock::now_secs().div_euclid(86_400),
            salt: fresh_salt(),
            seen_day: HashSet::new(),
            seen_hour: HashSet::new(),
        })
    }

    /// Database opened read-only by the `stats` command while the site is stopped.
    pub fn open_read(dir: &Path) -> Result<Database, String> {
        Database::open(dir.join("stats.redb")).map_err(|e| format!("stats db: {e}"))
    }

    fn roll(&mut self, now: i64) {
        let day = now.div_euclid(86_400);
        if day != self.day {
            self.day = day;
            self.salt = fresh_salt();
            self.seen_day.clear();
            self.seen_hour.clear();
        }
    }

    /// Counts one event. The address and User-Agent are used only for the hash.
    pub fn record(&mut self, ip: IpAddr, user_agent: &str, hit: &Hit<'_>) -> Result<(), String> {
        let now = clock::now_secs();
        self.roll(now);
        let date = clock::date(now);
        let hour = clock::hour(now);
        let mut keys: Vec<String> = Vec::new();
        if hit.event == "view" {
            let mut h = Sha256::new();
            h.update(self.salt);
            h.update(ip.to_string().as_bytes());
            h.update([0]);
            h.update(user_agent.as_bytes());
            let full = h.finalize();
            let mut id = [0u8; 16];
            id.copy_from_slice(&full[..16]);
            if self.seen_day.insert(id) {
                keys.push(format!("v|{date}"));
                keys.push(format!("dev|{date}|{}", ua::device(user_agent)));
                keys.push(format!("br|{date}|{}", ua::browser(user_agent)));
                keys.push(format!("os|{date}|{}", ua::os(user_agent)));
                if let Some(l) = hit.lang {
                    keys.push(format!("lang|{date}|{l}"));
                }
            }
            if self.seen_hour.insert((hour, id)) {
                keys.push(format!("vh|{date}|{hour:02}"));
            }
            keys.push(format!("p|{date}"));
            keys.push(format!("ph|{date}|{hour:02}"));
            if let Some(p) = hit.page {
                keys.push(format!("page|{date}|{p}"));
            }
            if let Some(r) = &hit.referrer {
                keys.push(format!("ref|{date}|{r}"));
            }
        } else {
            keys.push(match hit.detail {
                Some(d) => format!("ev|{date}|{}|{d}", hit.event),
                None => format!("ev|{date}|{}", hit.event),
            });
        }
        self.add(&keys)
    }

    fn add(&self, keys: &[String]) -> Result<(), String> {
        let w = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut t = w.open_table(COUNTERS).map_err(|e| e.to_string())?;
            for k in keys {
                let cur = t
                    .get(k.as_str())
                    .map_err(|e| e.to_string())?
                    .map_or(0, |v| v.value());
                t.insert(k.as_str(), cur + 1).map_err(|e| e.to_string())?;
            }
        }
        w.commit().map_err(|e| e.to_string())
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    /// The database answers reads.
    pub fn healthy(&self) -> bool {
        self.db
            .begin_read()
            .ok()
            .and_then(|r| r.open_table(COUNTERS).ok())
            .is_some()
    }
}

/// All counters, by key.
pub fn dump(db: &Database) -> Result<BTreeMap<String, u64>, String> {
    let r = db.begin_read().map_err(|e| e.to_string())?;
    let t = r.open_table(COUNTERS).map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for row in t.iter().map_err(|e| e.to_string())? {
        let (k, v) = row.map_err(|e| e.to_string())?;
        out.insert(k.value().to_string(), v.value());
    }
    Ok(out)
}

/// Totals sent by the bot; every visitor figure is a sum of daily figures.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct Summary {
    pub today: u64,
    pub this_month: u64,
    pub last_month: u64,
    pub total: u64,
    pub avg_30: f64,
    pub avg_all: f64,
    pub views_today: u64,
    pub views_total: u64,
    pub days: u64,
}

pub fn summary(counters: &BTreeMap<String, u64>, now: i64) -> Summary {
    let today = clock::date(now);
    let day_now = now.div_euclid(86_400);
    let (y, m, _) = clock::civil(day_now);
    let this_month = format!("{y:04}-{m:02}");
    let (ly, lm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
    let last_month = format!("{ly:04}-{lm:02}");
    let from_30 = day_now - 29;
    let mut s = Summary {
        today: 0,
        this_month: 0,
        last_month: 0,
        total: 0,
        avg_30: 0.0,
        avg_all: 0.0,
        views_today: 0,
        views_total: 0,
        days: 0,
    };
    let mut first: Option<i64> = None;
    let mut sum_30 = 0u64;
    for (k, v) in counters {
        let mut parts = k.split('|');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("v"), Some(d), None) => {
                s.total += v;
                if d == today {
                    s.today = *v;
                }
                if d.starts_with(&this_month) {
                    s.this_month += v;
                }
                if d.starts_with(&last_month) {
                    s.last_month += v;
                }
                if let Some(n) = clock::parse_date(d) {
                    first = Some(first.map_or(n, |f| f.min(n)));
                    if n >= from_30 {
                        sum_30 += v;
                    }
                }
            }
            (Some("p"), Some(d), None) => {
                s.views_total += v;
                if d == today {
                    s.views_today = *v;
                }
            }
            _ => {}
        }
    }
    let days_all = first.map_or(0, |f| day_now - f + 1).max(0) as u64;
    s.days = days_all;
    s.avg_all = if days_all > 0 {
        s.total as f64 / days_all as f64
    } else {
        0.0
    };
    let days_30 = days_all.clamp(1, 30);
    s.avg_30 = if days_all > 0 {
        sum_30 as f64 / days_30 as f64
    } else {
        0.0
    };
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_and_summary() {
        let dir = std::env::temp_dir().join(format!("sss-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut st = Stats::open(&dir).unwrap();
        let ip: IpAddr = "203.0.113.5".parse().unwrap();
        let ua = "Mozilla/5.0 (Windows NT 10.0) Chrome/130.0 Safari/537.36";
        let view = Hit {
            event: "view",
            page: Some("editor"),
            referrer: Some("example.org".into()),
            detail: None,
            lang: Some("ru"),
        };
        st.record(ip, ua, &view).unwrap();
        st.record(ip, ua, &view).unwrap();
        st.record("203.0.113.6".parse().unwrap(), ua, &view)
            .unwrap();
        st.record(
            ip,
            ua,
            &Hit {
                event: "open",
                page: None,
                referrer: None,
                detail: None,
                lang: None,
            },
        )
        .unwrap();
        let c = dump(st.db()).unwrap();
        let s = summary(&c, clock::now_secs());
        assert_eq!(s.today, 2);
        assert_eq!(s.views_today, 3);
        assert_eq!(s.total, 2);
        // Nothing about the visitor is stored: only aggregate keys.
        assert!(c.keys().all(|k| !k.contains("203.0.113")));
        drop(st);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
