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

//! The website: embedded files, security headers and the statistics endpoint.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use include_dir::{include_dir, Dir};
use serde::Deserialize;

use crate::clock;
use crate::config::Config;
use crate::stats::{self, Hit, Stats};
use crate::ua;

static SITE: Dir<'static> = include_dir!("$SITE_DIST");

const MAX_HIT_BODY: usize = 1024;

pub struct Shared {
    pub cfg: Config,
    pub stats: Mutex<Stats>,
    limiter: Mutex<Limiter>,
    traffic: Mutex<Traffic>,
    pub started: i64,
}

/// Requests and server errors per minute for the last minutes, for the watchdog.
#[derive(Default)]
struct Traffic {
    minutes: [(i64, u64, u64); 10],
}

impl Traffic {
    fn add(&mut self, error: bool) {
        let m = clock::now_secs() / 60;
        let slot = &mut self.minutes[(m % 10) as usize];
        if slot.0 != m {
            *slot = (m, 0, 0);
        }
        slot.1 += 1;
        if error {
            slot.2 += 1;
        }
    }

    /// (requests, errors) over the last `n` minutes.
    fn recent(&self, n: i64) -> (u64, u64) {
        let m = clock::now_secs() / 60;
        self.minutes
            .iter()
            .filter(|s| s.0 > m - n)
            .fold((0, 0), |a, s| (a.0 + s.1, a.1 + s.2))
    }
}

/// Events per address and in total, per minute. Addresses stay in memory
/// only for the current minute.
#[derive(Default)]
struct Limiter {
    minute: i64,
    total: u32,
    by_ip: HashMap<IpAddr, u32>,
}

impl Limiter {
    fn allow(&mut self, ip: IpAddr, per_ip: u32, total: u32) -> bool {
        let m = clock::now_secs() / 60;
        if m != self.minute {
            self.minute = m;
            self.total = 0;
            self.by_ip.clear();
        }
        let n = self.by_ip.entry(ip).or_insert(0);
        if *n >= per_ip || self.total >= total {
            return false;
        }
        *n += 1;
        self.total += 1;
        true
    }
}

pub fn shared(cfg: Config) -> Result<Arc<Shared>, String> {
    let stats = Stats::open(&cfg.data_dir)?;
    Ok(Arc::new(Shared {
        cfg,
        stats: Mutex::new(stats),
        limiter: Mutex::new(Limiter::default()),
        traffic: Mutex::new(Traffic::default()),
        started: clock::now_secs(),
    }))
}

pub fn router(state: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/hit", post(hit))
        .fallback(get(file).head(file))
        .layer(axum::middleware::from_fn_with_state(state.clone(), headers))
        .with_state(state)
}

/// Security headers on every answer, and the error counter.
async fn headers(
    State(st): State<Arc<Shared>>,
    req: Request,
    next: axum::middleware::Next,
) -> Response {
    let mut res = next.run(req).await;
    st.traffic
        .lock()
        .map(|mut t| t.add(res.status().is_server_error()))
        .ok();
    let h = res.headers_mut();
    let https = st.cfg.https();
    let mut csp = String::from(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; \
         style-src-attr 'unsafe-inline'; img-src 'self'; font-src 'self'; connect-src 'self'; \
         worker-src 'self'; manifest-src 'self'; base-uri 'none'; form-action 'none'; \
         frame-ancestors 'none'",
    );
    if https {
        csp.push_str("; upgrade-insecure-requests");
        h.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }
    if let Ok(v) = HeaderValue::from_str(&csp) {
        h.insert(header::CONTENT_SECURITY_POLICY, v);
    }
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    h.insert(
        "permissions-policy",
        HeaderValue::from_static(
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), interest-cohort=(), browsing-topics=()",
        ),
    );
    h.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    h.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    h.insert(
        "x-permitted-cross-domain-policies",
        HeaderValue::from_static("none"),
    );
    res
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

fn lang_from(headers: &HeaderMap) -> &'static str {
    let accept = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // The first listed language decides, like the browser setting.
    let first = accept
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if first == "ru" || first.starts_with("ru-") {
        "ru"
    } else {
        "en"
    }
}

async fn file(method: Method, headers: HeaderMap, req: Request) -> Response {
    let path = req.uri().path().to_string();
    if path == "/" {
        let lang = lang_from(&headers);
        return Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, format!("/{lang}/"))
            .header(header::VARY, "Accept-Language")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::empty())
            .unwrap_or_default();
    }
    let rel = path.trim_start_matches('/');
    if rel.contains("..") || rel.contains('\\') || rel.contains('\0') {
        return not_found("en", method);
    }
    // Pages without the trailing slash go to the canonical address.
    if !rel.is_empty()
        && !rel.contains('.')
        && !rel.ends_with('/')
        && SITE.get_file(format!("{rel}/index.html")).is_some()
    {
        return Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(header::LOCATION, format!("/{rel}/"))
            .body(Body::empty())
            .unwrap_or_default();
    }
    let name = if rel.ends_with('/') || rel.is_empty() {
        format!("{rel}index.html")
    } else {
        rel.to_string()
    };
    let Some(f) = SITE.get_file(&name) else {
        let lang = if rel.starts_with("ru/") { "ru" } else { "en" };
        return not_found(lang, method);
    };
    serve_file(&name, f.contents(), &headers, method, StatusCode::OK)
}

fn not_found(lang: &str, method: Method) -> Response {
    let name = format!("{lang}/404.html");
    match SITE.get_file(&name) {
        Some(f) => serve_file(
            &name,
            f.contents(),
            &HeaderMap::new(),
            method,
            StatusCode::NOT_FOUND,
        ),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

fn serve_file(
    name: &str,
    raw: &'static [u8],
    headers: &HeaderMap,
    method: Method,
    status: StatusCode,
) -> Response {
    let accept = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (body, enc) = [("br", ".br"), ("gzip", ".gz")]
        .iter()
        .find(|(e, _)| {
            accept
                .split(',')
                .any(|a| a.trim().split(';').next() == Some(e))
        })
        .and_then(|(e, ext)| {
            SITE.get_file(format!("{name}{ext}"))
                .map(|f| (f.contents(), Some(*e)))
        })
        .unwrap_or((raw, None));
    let immutable =
        name.starts_with("assets/") && name.rsplit('/').next().is_some_and(|f| f.contains('-'));
    let cache = if immutable {
        "public, max-age=31536000, immutable"
    } else if name.ends_with(".html") || name == "sw.js" {
        "no-cache"
    } else {
        "public, max-age=3600"
    };
    let mut b = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type(name))
        .header(header::CACHE_CONTROL, cache)
        .header(header::VARY, "Accept-Encoding")
        .header(header::CONTENT_LENGTH, body.len());
    if let Some(e) = enc {
        b = b.header(header::CONTENT_ENCODING, e);
    }
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(Bytes::from_static(body))
    };
    b.body(body).unwrap_or_default()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HitBody {
    e: String,
    #[serde(default)]
    p: String,
    #[serde(default)]
    r: String,
    #[serde(default)]
    d: String,
    #[serde(default)]
    l: String,
}

/// Address of the visitor: from the proxy header only when the proxy is trusted.
fn client_ip(peer: SocketAddr, headers: &HeaderMap, trusted: &[IpAddr]) -> IpAddr {
    if !trusted.contains(&peer.ip()) {
        return peer.ip();
    }
    let real = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok());
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .and_then(|v| v.trim().parse().ok());
    real.or(forwarded).unwrap_or(peer.ip())
}

fn page_of(path: &str) -> Option<&'static str> {
    let rest = path
        .strip_prefix("/en/")
        .or_else(|| path.strip_prefix("/ru/"))?;
    let name = rest.trim_end_matches('/');
    if name.is_empty() {
        return Some("editor");
    }
    stats::PAGES.iter().find(|p| **p == name).copied()
}

fn clean_domain(r: &str) -> Option<String> {
    let r = r.trim().trim_end_matches('.').to_ascii_lowercase();
    let ok = !r.is_empty()
        && r.len() <= 253
        && r.contains('.')
        && r.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    ok.then_some(r)
}

async fn hit(
    State(st): State<Arc<Shared>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // Counting must never disturb the editor: every refusal is a silent 204.
    let done = StatusCode::NO_CONTENT;
    if body.len() > MAX_HIT_BODY {
        return StatusCode::PAYLOAD_TOO_LARGE;
    }
    let opt_out = headers.get("sec-gpc").is_some_and(|v| v.as_bytes() == b"1")
        || headers.get("dnt").is_some_and(|v| v.as_bytes() == b"1");
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if opt_out || ua::is_robot(user_agent) {
        return done;
    }
    let Ok(b) = serde_json::from_slice::<HitBody>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    let event: &str = match b.e.as_str() {
        "view" => "view",
        "open" => "open",
        "save" => "save",
        "error" => "error",
        "desktop" => "desktop",
        _ => return StatusCode::BAD_REQUEST,
    };
    let detail = match event {
        "error" => Some(
            stats::ERRORS
                .iter()
                .find(|e| **e == b.d)
                .copied()
                .unwrap_or("other"),
        ),
        _ => None,
    };
    let page = page_of(&b.p);
    if event == "view" && page.is_none() {
        return done;
    }
    let host = st.cfg.site_host();
    let referrer = clean_domain(&b.r).filter(|r| *r != host);
    let lang = match b.l.as_str() {
        "ru" => Some("ru"),
        "en" => Some("en"),
        _ => None,
    };
    let ip = client_ip(peer, &headers, &st.cfg.trusted_proxies);
    let allowed = st
        .limiter
        .lock()
        .map(|mut l| {
            l.allow(
                ip,
                st.cfg.limits.hits_per_minute,
                st.cfg.limits.hits_per_minute_total,
            )
        })
        .unwrap_or(false);
    if !allowed {
        return StatusCode::TOO_MANY_REQUESTS;
    }
    let h = Hit {
        event,
        page,
        referrer,
        detail,
        lang,
    };
    if let Ok(mut s) = st.stats.lock() {
        if let Err(e) = s.record(ip, user_agent, &h) {
            eprintln!("stats: {e}");
        }
    }
    done
}

/// Local endpoints for the watchdog and the `stats` command.
pub fn admin_router(state: Arc<Shared>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/summary", get(summary))
        .route("/dump", get(dump))
        .with_state(state)
}

fn local(peer: &SocketAddr) -> bool {
    peer.ip().is_loopback()
}

async fn health(
    State(st): State<Arc<Shared>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Response {
    if !local(&peer) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let db_ok = st.stats.lock().map(|s| s.healthy()).unwrap_or(false);
    let (requests, errors) = st.traffic.lock().map(|t| t.recent(5)).unwrap_or((0, 0));
    axum::Json(serde_json::json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "started": st.started,
        "dbOk": db_ok,
        "requests5m": requests,
        "errors5m": errors,
        "site": SITE.get_file("en/index.html").is_some(),
    }))
    .into_response()
}

async fn summary(
    State(st): State<Arc<Shared>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Response {
    if !local(&peer) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let counters = st
        .stats
        .lock()
        .map_err(|e| e.to_string())
        .and_then(|s| stats::dump(s.db()));
    match counters {
        Ok(c) => axum::Json(stats::summary(&c, clock::now_secs())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn dump(
    State(st): State<Arc<Shared>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Response {
    if !local(&peer) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match st
        .stats
        .lock()
        .map_err(|e| e.to_string())
        .and_then(|s| stats::dump(s.db()))
    {
        Ok(c) => axum::Json(c).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn run(cfg: Config) -> Result<(), String> {
    let st = shared(cfg)?;
    let site = tokio::net::TcpListener::bind(st.cfg.listen)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", st.cfg.listen))?;
    let admin = tokio::net::TcpListener::bind(st.cfg.admin_listen)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", st.cfg.admin_listen))?;
    println!(
        "shelter-site {} listening on {} (admin {})",
        env!("CARGO_PKG_VERSION"),
        st.cfg.listen,
        st.cfg.admin_listen
    );
    let app = router(st.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let adm = admin_router(st).into_make_service_with_connect_info::<SocketAddr>();
    let a = tokio::spawn(async move { axum::serve(admin, adm).await });
    let s = axum::serve(site, app).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    });
    tokio::select! {
        r = s => r.map_err(|e| e.to_string()),
        r = a => Err(format!("admin server stopped: {r:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_checks() {
        assert_eq!(page_of("/ru/"), Some("editor"));
        assert_eq!(page_of("/en/help/"), Some("help"));
        assert_eq!(page_of("/en/../etc/"), None);
        assert_eq!(page_of("/de/"), None);
        assert_eq!(clean_domain("Example.org"), Some("example.org".into()));
        assert_eq!(clean_domain("x<script>.org"), None);
        assert_eq!(clean_domain("localhost"), None);
        let proxy: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let other: SocketAddr = "198.51.100.1:5000".parse().unwrap();
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", HeaderValue::from_static("203.0.113.9"));
        let trusted = vec!["127.0.0.1".parse().unwrap()];
        assert_eq!(client_ip(proxy, &h, &trusted).to_string(), "203.0.113.9");
        assert_eq!(client_ip(other, &h, &trusted).to_string(), "198.51.100.1");
        let mut l = Limiter::default();
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        assert!(l.allow(ip, 2, 100) && l.allow(ip, 2, 100) && !l.allow(ip, 2, 100));
    }
}
