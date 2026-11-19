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

//! Watchdog: a separate process that checks the site and reports to the
//! administrator in Telegram. Its state is a small JSON file next to the
//! statistics; outages are appended to `incidents.log`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clock::{self, span_ru, stamp};
use crate::config::Config;
use crate::stats::Summary;
use crate::telegram::Bot;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Last moment the watchdog was alive, to spot a whole-server outage.
    pub last_beat: i64,
    /// Problems in progress: since, last alert, current value.
    pub problems: BTreeMap<String, Problem>,
    /// Failed checks in a row, before a problem is confirmed.
    pub streak: BTreeMap<String, u32>,
    /// Messages Telegram has not accepted yet.
    pub outbox: Vec<String>,
    pub mute_until: i64,
    pub offset: i64,
    pub service_started: i64,
    pub last_outage: Option<(i64, i64)>,
    pub cert_days: Option<i64>,
    pub cert_checked: i64,
    pub disk_free: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Problem {
    pub since: i64,
    pub alerted: i64,
    pub value: String,
}

fn state_path(cfg: &Config) -> PathBuf {
    cfg.data_dir.join("watch.json")
}

pub fn load_state(cfg: &Config) -> State {
    std::fs::read_to_string(state_path(cfg))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_state(cfg: &Config, st: &State) {
    let path = state_path(cfg);
    let tmp = path.with_extension("json.tmp");
    if let Ok(text) = serde_json::to_string_pretty(st) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// One line per outage, readable by the `stats` command.
fn journal(cfg: &Config, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg.data_dir.join("incidents.log"))
    {
        let _ = writeln!(f, "{} {line}", stamp(clock::now_secs()));
    }
}

fn agent(timeout: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout)))
        .user_agent("shelter-watch")
        .http_status_as_error(false)
        .build()
        .new_agent()
}

fn get_json(url: &str) -> Option<Value> {
    let mut res = agent(5).get(url).call().ok()?;
    if res.status() != 200 {
        return None;
    }
    res.body_mut().read_json().ok()
}

/// Result of one round of checks: problem key -> current value, if failing.
struct Round {
    failing: BTreeMap<String, String>,
    health: Option<Value>,
}

fn service_active(name: &str) -> Option<bool> {
    if name.is_empty() || !cfg!(target_os = "linux") {
        return None;
    }
    let out = std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", name])
        .status()
        .ok()?;
    Some(out.success())
}

#[cfg(unix)]
fn disk_free_percent(path: &Path) -> Option<f64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    // statvfs only fills the struct we pass in.
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 || s.f_blocks == 0 {
        return None;
    }
    Some(s.f_bavail as f64 / s.f_blocks as f64 * 100.0)
}

#[cfg(not(unix))]
fn disk_free_percent(_path: &Path) -> Option<f64> {
    None
}

/// Days until the public certificate expires.
fn cert_days(host: &str) -> Option<i64> {
    use std::net::TcpStream;
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .ok()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(host.to_string()).ok()?;
    let mut conn = rustls::ClientConnection::new(Arc::new(config), name).ok()?;
    let mut sock = TcpStream::connect((host, 443)).ok()?;
    sock.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    sock.set_write_timeout(Some(Duration::from_secs(10))).ok()?;
    let started = Instant::now();
    while conn.is_handshaking() {
        if started.elapsed() > Duration::from_secs(15) {
            return None;
        }
        conn.complete_io(&mut sock).ok()?;
    }
    let cert = conn.peer_certificates()?.first()?.clone();
    let (_, parsed) = x509_parser::parse_x509_certificate(cert.as_ref()).ok()?;
    let not_after = parsed.validity().not_after.timestamp();
    Some((not_after - clock::now_secs()).div_euclid(86_400))
}

fn check(cfg: &Config, st: &mut State) -> Round {
    let w = &cfg.watch;
    let mut failing = BTreeMap::new();
    if let Some(false) = service_active(&w.service) {
        failing.insert("service".to_string(), "служба остановлена".to_string());
    }
    let started = Instant::now();
    let health = get_json(&format!("http://{}/health", cfg.admin_listen));
    let local_ms = started.elapsed().as_millis() as u64;
    match &health {
        None => {
            failing.insert("local".into(), "сайт не отвечает локально".into());
        }
        Some(h) => {
            if local_ms > w.slow_ms {
                failing.insert("local_slow".into(), format!("{local_ms} мс"));
            }
            if h.get("dbOk").and_then(Value::as_bool) != Some(true) {
                failing.insert("db".into(), "хранилище статистики недоступно".into());
            }
            let req = h.get("requests5m").and_then(Value::as_u64).unwrap_or(0);
            let err = h.get("errors5m").and_then(Value::as_u64).unwrap_or(0);
            if req >= w.error_min_requests && err as f64 / req as f64 > w.error_share {
                failing.insert("errors".into(), format!("{err} из {req} за 5 мин"));
            }
        }
    }
    let started = Instant::now();
    match agent(15).get(&cfg.site_url).call() {
        Ok(r) if r.status().as_u16() < 400 => {
            let ms = started.elapsed().as_millis() as u64;
            if ms > w.slow_ms {
                failing.insert("public_slow".into(), format!("{ms} мс"));
            }
        }
        Ok(r) => {
            failing.insert("public".into(), format!("ответ {}", r.status().as_u16()));
        }
        Err(_) => {
            failing.insert("public".into(), "не открывается".into());
        }
    }
    st.disk_free = disk_free_percent(&w.disk_path);
    if let Some(free) = st.disk_free {
        if free < w.disk_free_percent {
            failing.insert("disk".into(), format!("свободно {free:.1}%"));
        }
    }
    let now = clock::now_secs();
    if cfg.https() && now - st.cert_checked > 6 * 3600 {
        st.cert_checked = now;
        st.cert_days = cert_days(&cfg.site_host());
    }
    if let Some(d) = st.cert_days {
        if d < w.cert_warn_days {
            failing.insert("cert".into(), format!("осталось {d} д"));
        }
    }
    Round { failing, health }
}

fn label(key: &str) -> &'static str {
    match key {
        "service" => "Служба сайта",
        "local" => "Сайт на сервере",
        "local_slow" => "Медленный ответ сайта на сервере",
        "db" => "Хранилище статистики",
        "errors" => "Ошибки сервера",
        "public" => "Сайт по публичному адресу",
        "public_slow" => "Медленный ответ по публичному адресу",
        "disk" => "Место на диске",
        "cert" => "Сертификат",
        _ => "Проверка",
    }
}

/// Turns a round into alerts: a problem needs `confirm_checks` failures in a
/// row, repeats at most every `repeat_minutes`, and recovery is reported once.
fn evaluate(cfg: &Config, st: &mut State, round: &Round, now: i64) -> Vec<String> {
    let mut out = Vec::new();
    let keys: Vec<String> = st
        .problems
        .keys()
        .chain(round.failing.keys())
        .cloned()
        .collect();
    for key in keys {
        match round.failing.get(&key) {
            Some(value) => {
                let n = st.streak.entry(key.clone()).or_insert(0);
                *n += 1;
                let confirmed = *n >= cfg.watch.confirm_checks;
                let repeat = cfg.watch.repeat_minutes as i64 * 60;
                match st.problems.get_mut(&key) {
                    Some(p) => {
                        p.value = value.clone();
                        if now - p.alerted >= repeat {
                            p.alerted = now;
                            out.push(format!(
                                "Проблема: {} — всё ещё есть ({}), с {} ({}).",
                                label(&key),
                                value,
                                stamp(p.since),
                                span_ru(now - p.since)
                            ));
                        }
                    }
                    None if confirmed => {
                        let since = now - (cfg.watch.interval_secs as i64) * (i64::from(*n) - 1);
                        st.problems.insert(
                            key.clone(),
                            Problem {
                                since,
                                alerted: now,
                                value: value.clone(),
                            },
                        );
                        out.push(format!(
                            "Проблема: {} — {} (с {}).",
                            label(&key),
                            value,
                            stamp(since)
                        ));
                    }
                    None => {}
                }
            }
            None => {
                st.streak.remove(&key);
                if let Some(p) = st.problems.remove(&key) {
                    let msg = format!(
                        "Восстановлено: {}. Сбой длился {} (с {}).",
                        label(&key),
                        span_ru(now - p.since),
                        stamp(p.since)
                    );
                    journal(cfg, &format!("{key} {} -> {}", stamp(p.since), stamp(now)));
                    out.push(msg);
                }
            }
        }
    }
    if let Some(started) = round
        .health
        .as_ref()
        .and_then(|h| h.get("started"))
        .and_then(Value::as_i64)
    {
        if st.service_started != 0 && started != st.service_started {
            out.push(format!("Служба сайта перезапущена в {}.", stamp(started)));
            journal(cfg, &format!("restart {}", stamp(started)));
        }
        st.service_started = started;
    }
    out
}

fn summary_from_site(cfg: &Config) -> Option<Summary> {
    get_json(&format!("http://{}/summary", cfg.admin_listen))
        .and_then(|v| serde_json::from_value(v).ok())
}

fn stats_text(cfg: &Config) -> String {
    match summary_from_site(cfg) {
        None => "Статистика недоступна: сайт не отвечает.".into(),
        Some(s) => format!(
            "Посетители (уникальные за каждые сутки UTC, суммы суточных значений):\n\
             • сегодня: {}\n• этот месяц: {}\n• прошлый месяц: {}\n• всего: {}\n\
             • в среднем за день: {:.1} (30 дней), {:.1} (всё время, {} дн.)\n\
             Просмотры: сегодня {}, всего {}.",
            s.today,
            s.this_month,
            s.last_month,
            s.total,
            s.avg_30,
            s.avg_all,
            s.days,
            s.views_today,
            s.views_total
        ),
    }
}

fn status_text(cfg: &Config, st: &State) -> String {
    let health = get_json(&format!("http://{}/health", cfg.admin_listen));
    let now = clock::now_secs();
    let mut lines = Vec::new();
    match &health {
        Some(h) => {
            let started = h.get("started").and_then(Value::as_i64).unwrap_or(now);
            let version = h.get("version").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!(
                "Сайт работает. Версия {version}, без перерыва {}.",
                span_ru(now - started)
            ));
        }
        None => lines.push("Сайт не отвечает на сервере.".into()),
    }
    if st.problems.is_empty() {
        lines.push("Проблем нет.".into());
    } else {
        for (k, p) in &st.problems {
            lines.push(format!(
                "Проблема: {} — {} (с {}).",
                label(k),
                p.value,
                stamp(p.since)
            ));
        }
    }
    lines.push(match st.last_outage {
        Some((a, b)) => format!(
            "Последний сбой: {} — {} ({}).",
            stamp(a),
            stamp(b),
            span_ru(b - a)
        ),
        None => "Сбоев не было.".into(),
    });
    if let Some(d) = st.disk_free {
        lines.push(format!("Свободно на диске: {d:.1}%."));
    }
    if let Some(d) = st.cert_days {
        lines.push(format!("Сертификат истекает через {d} д."));
    }
    if st.mute_until > now {
        lines.push(format!("Оповещения выключены до {}.", stamp(st.mute_until)));
    }
    lines.join("\n")
}

const HELP: &str = "Команды:\n/stats — краткая статистика\n/status — состояние сайта\n\
/mute 2h — приостановить оповещения (минуты: 30m, часы: 2h, дни: 1d; без числа — 1 час)\n\
/unmute — включить оповещения\n/help — эта справка";

/// Parses "30m", "2h", "1d" or plain minutes.
fn parse_span(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(3600);
    }
    let (n, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: i64 = n.parse().ok()?;
    let k = match unit.trim() {
        "" | "m" | "м" | "min" | "мин" => 60,
        "h" | "ч" => 3600,
        "d" | "д" => 86_400,
        _ => return None,
    };
    Some((n * k).clamp(60, 30 * 86_400))
}

fn answer(cfg: &Config, state: &Mutex<State>, text: &str) -> String {
    let mut parts = text.trim().splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").split('@').next().unwrap_or("");
    let arg = parts.next().unwrap_or("");
    match cmd {
        "/stats" => stats_text(cfg),
        "/status" => state
            .lock()
            .map(|s| status_text(cfg, &s))
            .unwrap_or_default(),
        "/mute" => match parse_span(arg) {
            Some(secs) => {
                let until = clock::now_secs() + secs;
                if let Ok(mut s) = state.lock() {
                    s.mute_until = until;
                    save_state(cfg, &s);
                }
                format!("Оповещения приостановлены до {}.", stamp(until))
            }
            None => "Не понял срок. Пример: /mute 2h".into(),
        },
        "/unmute" => {
            if let Ok(mut s) = state.lock() {
                s.mute_until = 0;
                save_state(cfg, &s);
            }
            "Оповещения включены.".into()
        }
        _ => HELP.into(),
    }
}

fn flush(cfg: &Config, bot: Option<&Bot>, state: &Mutex<State>) {
    let Some(bot) = bot else { return };
    let pending: Vec<String> = match state.lock() {
        Ok(s) => s.outbox.clone(),
        Err(_) => return,
    };
    let mut sent = 0;
    for m in &pending {
        if bot.send(m).is_err() {
            break;
        }
        sent += 1;
    }
    if sent > 0 {
        if let Ok(mut s) = state.lock() {
            let n = sent.min(s.outbox.len());
            s.outbox.drain(..n);
            save_state(cfg, &s);
        }
    }
}

pub fn run(cfg: Config) -> Result<(), String> {
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| format!("cannot create {}: {e}", cfg.data_dir.display()))?;
    let state = Arc::new(Mutex::new(load_state(&cfg)));
    let bot = Bot::new(&cfg.telegram.token, cfg.telegram.admin_id).map(Arc::new);
    if bot.is_none() {
        eprintln!("telegram is not configured: alerts are only written to incidents.log");
    }
    let interval = cfg.watch.interval_secs as i64;
    {
        let now = clock::now_secs();
        let mut s = state.lock().map_err(|e| e.to_string())?;
        // A long silence means the whole server (and this watchdog) was down.
        if s.last_beat > 0 && now - s.last_beat > 3 * interval {
            let (from, to) = (s.last_beat, now);
            s.last_outage = Some((from, to));
            s.outbox.push(format!(
                "Сервер вернулся после отключения. Простой: {} — {} ({}).",
                stamp(from),
                stamp(to),
                span_ru(to - from)
            ));
            journal(
                &cfg,
                &format!("server-down {} -> {}", stamp(from), stamp(to)),
            );
        }
        s.last_beat = now;
        save_state(&cfg, &s);
    }
    if let Some(b) = bot.clone() {
        let cfg = cfg.clone();
        let state = state.clone();
        std::thread::spawn(move || loop {
            let offset = state.lock().map(|s| s.offset).unwrap_or(0);
            match b.poll(offset) {
                Ok((cmds, next)) => {
                    for c in cmds {
                        let reply = answer(&cfg, &state, &c.text);
                        if b.send(&reply).is_err() {
                            if let Ok(mut s) = state.lock() {
                                s.outbox.push(reply);
                            }
                        }
                    }
                    if let Ok(mut s) = state.lock() {
                        s.offset = next;
                        save_state(&cfg, &s);
                    }
                }
                Err(e) => {
                    eprintln!("telegram: {e}");
                    std::thread::sleep(Duration::from_secs(30));
                }
            }
        });
    }
    println!(
        "shelter-site watch {} every {} s",
        env!("CARGO_PKG_VERSION"),
        interval
    );
    loop {
        let tick = Instant::now();
        let mut s = state.lock().map_err(|e| e.to_string())?.clone_state();
        let round = check(&cfg, &mut s);
        let now = clock::now_secs();
        let msgs = evaluate(&cfg, &mut s, &round, now);
        s.last_beat = now;
        {
            let mut shared = state.lock().map_err(|e| e.to_string())?;
            // Commands may have changed these while the checks ran.
            s.mute_until = shared.mute_until;
            s.offset = shared.offset;
            s.outbox = std::mem::take(&mut shared.outbox);
            for m in msgs {
                eprintln!("{m}");
                if s.mute_until <= now || m.starts_with("Восстановлено") {
                    s.outbox.push(m);
                }
            }
            if s.outbox.len() > 200 {
                let drop = s.outbox.len() - 200;
                s.outbox.drain(..drop);
            }
            *shared = s;
            save_state(&cfg, &shared);
        }
        flush(&cfg, bot.as_deref(), &state);
        let wait = Duration::from_secs(cfg.watch.interval_secs).saturating_sub(tick.elapsed());
        std::thread::sleep(wait);
    }
}

impl State {
    fn clone_state(&self) -> State {
        serde_json::from_value(serde_json::to_value(self).unwrap_or_default()).unwrap_or_default()
    }
}

/// Lines of the outage journal, for the full report.
pub fn incidents(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("incidents.log"))
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alerts_need_confirmation_and_recover() {
        let cfg = Config {
            data_dir: std::env::temp_dir().join(format!("sss-watch-{}", std::process::id())),
            ..Config::default()
        };
        std::fs::create_dir_all(&cfg.data_dir).unwrap();
        let mut st = State::default();
        let failing = |v: &str| Round {
            failing: [("public".to_string(), v.to_string())]
                .into_iter()
                .collect(),
            health: None,
        };
        let ok = Round {
            failing: BTreeMap::new(),
            health: None,
        };
        assert!(evaluate(&cfg, &mut st, &failing("x"), 1000).is_empty());
        let m = evaluate(&cfg, &mut st, &failing("x"), 1060);
        assert_eq!(m.len(), 1);
        assert!(evaluate(&cfg, &mut st, &failing("x"), 1120).is_empty());
        let m = evaluate(&cfg, &mut st, &ok, 1180);
        assert!(m[0].starts_with("Восстановлено"));
        assert!(evaluate(&cfg, &mut st, &ok, 1240).is_empty());
        let _ = std::fs::remove_dir_all(&cfg.data_dir);
    }

    #[test]
    fn spans_and_commands() {
        assert_eq!(parse_span("2h"), Some(7200));
        assert_eq!(parse_span(""), Some(3600));
        assert_eq!(parse_span("30m"), Some(1800));
        assert_eq!(parse_span("abc"), None);
    }
}
