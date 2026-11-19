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

//! Settings file (TOML). Every field has a default except the secrets.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Public address of the site, without a trailing slash.
    pub site_url: String,
    /// Where the site listens; only a local address.
    pub listen: SocketAddr,
    /// Local address for the watchdog and the `stats` command.
    pub admin_listen: SocketAddr,
    /// Folder for the statistics database and the watchdog state.
    pub data_dir: PathBuf,
    /// Proxies whose `X-Forwarded-For` / `X-Real-IP` headers are trusted.
    pub trusted_proxies: Vec<IpAddr>,
    pub limits: Limits,
    pub watch: Watch,
    pub telegram: Telegram,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    /// Statistics events accepted from one address per minute.
    pub hits_per_minute: u32,
    /// Statistics events accepted from everyone per minute.
    pub hits_per_minute_total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Watch {
    /// systemd unit of the site; empty skips the check.
    pub service: String,
    pub interval_secs: u64,
    /// Answer slower than this counts as a problem.
    pub slow_ms: u64,
    /// Share of server errors (5xx) over the last minutes that counts as a problem.
    pub error_share: f64,
    /// Fewer requests than this are too few to judge the error share.
    pub error_min_requests: u64,
    /// Free disk space below this percentage is a problem.
    pub disk_free_percent: f64,
    pub disk_path: PathBuf,
    /// A certificate expiring sooner than this is a problem.
    pub cert_warn_days: i64,
    /// Checks in a row that must fail before an alert.
    pub confirm_checks: u32,
    /// Shortest time between two alerts about the same problem.
    pub repeat_minutes: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Telegram {
    pub token: String,
    pub admin_id: i64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            site_url: "https://shelter.mahiron.online".into(),
            listen: SocketAddr::from(([127, 0, 0, 1], 8080)),
            admin_listen: SocketAddr::from(([127, 0, 0, 1], 8081)),
            data_dir: PathBuf::from("/var/lib/shelter-site"),
            trusted_proxies: vec![IpAddr::from([127, 0, 0, 1]), "::1".parse().expect("ipv6")],
            limits: Limits::default(),
            watch: Watch::default(),
            telegram: Telegram::default(),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            hits_per_minute: 60,
            hits_per_minute_total: 6000,
        }
    }
}

impl Default for Watch {
    fn default() -> Self {
        Watch {
            service: "shelter-site.service".into(),
            interval_secs: 60,
            slow_ms: 3000,
            error_share: 0.05,
            error_min_requests: 20,
            disk_free_percent: 10.0,
            disk_path: PathBuf::from("/var/lib/shelter-site"),
            cert_warn_days: 14,
            confirm_checks: 2,
            repeat_minutes: 60,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).map_err(|e| format!("bad config {}: {e}", path.display()))?;
        cfg.check()?;
        Ok(cfg)
    }

    fn check(&self) -> Result<(), String> {
        if !self.listen.ip().is_loopback() {
            return Err("listen must be a local (loopback) address behind the proxy".into());
        }
        if !self.admin_listen.ip().is_loopback() {
            return Err("admin_listen must be a local (loopback) address".into());
        }
        if !(self.site_url.starts_with("https://") || self.site_url.starts_with("http://")) {
            return Err("site_url must start with https://".into());
        }
        if self.watch.interval_secs < 10 {
            return Err("watch.interval_secs must be at least 10".into());
        }
        Ok(())
    }

    pub fn https(&self) -> bool {
        self.site_url.starts_with("https://")
    }

    pub fn site_host(&self) -> String {
        self.site_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split(['/', ':'])
            .next()
            .unwrap_or_default()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_checks() {
        let c: Config = toml::from_str("").unwrap();
        assert!(c.check().is_ok());
        assert_eq!(c.site_host(), "shelter.mahiron.online");
        let bad: Config = toml::from_str("listen = \"0.0.0.0:8080\"").unwrap();
        assert!(bad.check().is_err());
        assert!(toml::from_str::<Config>("unknown = 1").is_err());
    }
}
