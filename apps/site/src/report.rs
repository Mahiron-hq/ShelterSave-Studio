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

//! Full statistics for the administrator, on the server only.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde_json::{json, Value};

use crate::clock;
use crate::config::Config;
use crate::stats;
use crate::watch;

pub struct Options {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub format: String,
}

/// Counters from the running site, or from the database when the site is stopped.
fn counters(cfg: &Config) -> Result<BTreeMap<String, u64>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .new_agent();
    if let Ok(mut r) = agent
        .get(&format!("http://{}/dump", cfg.admin_listen))
        .call()
    {
        if let Ok(v) = r.body_mut().read_json::<BTreeMap<String, u64>>() {
            return Ok(v);
        }
    }
    let db = stats::Stats::open_read(&cfg.data_dir)?;
    stats::dump(&db)
}

fn in_range(date: &str, o: &Options) -> bool {
    let Some(d) = clock::parse_date(date) else {
        return false;
    };
    o.from.is_none_or(|f| d >= f) && o.to.is_none_or(|t| d <= t)
}

/// Rows (section, date, key, value) inside the period.
fn rows(c: &BTreeMap<String, u64>, o: &Options) -> Vec<(String, String, String, u64)> {
    let mut out = Vec::new();
    for (k, v) in c {
        let parts: Vec<&str> = k.split('|').collect();
        let (section, date, key) = match parts.as_slice() {
            ["v", d] => ("visitors_day", *d, String::new()),
            ["p", d] => ("views_day", *d, String::new()),
            ["vh", d, h] => ("visitors_hour", *d, (*h).to_string()),
            ["ph", d, h] => ("views_hour", *d, (*h).to_string()),
            ["page", d, p] => ("views_page", *d, (*p).to_string()),
            ["ref", d, r] => ("referrer", *d, (*r).to_string()),
            ["dev", d, x] => ("device", *d, (*x).to_string()),
            ["br", d, x] => ("browser", *d, (*x).to_string()),
            ["os", d, x] => ("os", *d, (*x).to_string()),
            ["lang", d, x] => ("language", *d, (*x).to_string()),
            ["ev", d, e] => ("event", *d, (*e).to_string()),
            ["ev", d, e, x] => ("event", *d, format!("{e}:{x}")),
            _ => continue,
        };
        if in_range(date, o) {
            out.push((section.to_string(), date.to_string(), key, *v));
        }
    }
    out
}

pub fn run(cfg: &Config, o: &Options) -> Result<String, String> {
    let c = counters(cfg)?;
    let rows = rows(&c, o);
    let summary = stats::summary(&c, clock::now_secs());
    let incidents = watch::incidents(&cfg.data_dir);
    let watch_state = watch::load_state(cfg);
    match o.format.as_str() {
        "json" => {
            let list: Vec<Value> = rows
                .iter()
                .map(|(s, d, k, v)| json!({ "section": s, "date": d, "key": k, "value": v }))
                .collect();
            serde_json::to_string_pretty(&json!({
                "summary": summary,
                "rows": list,
                "incidents": incidents,
                "lastOutage": watch_state.last_outage,
            }))
            .map_err(|e| e.to_string())
        }
        "csv" => {
            let mut s = String::from("section,date,key,value\n");
            for (sec, d, k, v) in rows {
                let k = k.replace('"', "\"\"");
                let _ = writeln!(s, "{sec},{d},\"{k}\",{v}");
            }
            Ok(s)
        }
        _ => {
            let mut s = String::new();
            let _ = writeln!(
                s,
                "Visitors are counted once per UTC day; month and total figures are sums of days."
            );
            let _ = writeln!(
                s,
                "Today {} | this month {} | last month {} | total {} | per day: {:.1} (30 days), {:.1} (all {} days)",
                summary.today, summary.this_month, summary.last_month, summary.total, summary.avg_30, summary.avg_all, summary.days
            );
            let _ = writeln!(
                s,
                "Views today {} | total {}",
                summary.views_today, summary.views_total
            );
            let mut by: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
            for (sec, d, k, v) in &rows {
                let key = if k.is_empty() {
                    d.clone()
                } else if sec.ends_with("_hour") {
                    format!("{d} {k}h")
                } else {
                    k.clone()
                };
                *by.entry(sec.clone()).or_default().entry(key).or_default() += v;
            }
            for (sec, list) in by {
                let _ = writeln!(s, "\n[{sec}]");
                let mut list: Vec<_> = list.into_iter().collect();
                if !sec.ends_with("_day") && !sec.ends_with("_hour") {
                    list.sort_by_key(|x| std::cmp::Reverse(x.1));
                }
                for (k, v) in list {
                    let _ = writeln!(s, "  {k:<40} {v}");
                }
            }
            let _ = writeln!(s, "\n[incidents]");
            for l in incidents {
                let _ = writeln!(s, "  {l}");
            }
            Ok(s)
        }
    }
}
