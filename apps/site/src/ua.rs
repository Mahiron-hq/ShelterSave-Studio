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

//! Coarse groups from a User-Agent string: no versions, nothing kept.

/// Robots, link previews and service checks are not counted.
pub fn is_robot(ua: &str) -> bool {
    let l = ua.to_ascii_lowercase();
    ua.is_empty()
        || [
            "bot",
            "crawl",
            "spider",
            "slurp",
            "preview",
            "monitor",
            "curl",
            "wget",
            "python",
            "headless",
            "lighthouse",
            "pagespeed",
            "httpclient",
            "java/",
            "go-http",
            "okhttp",
            "facebookexternalhit",
            "embedly",
            "shelter-watch",
            "uptime",
            "check",
        ]
        .iter()
        .any(|w| l.contains(w))
}

pub fn device(ua: &str) -> &'static str {
    if ua.contains("iPad")
        || (ua.contains("Android") && !ua.contains("Mobile"))
        || ua.contains("Tablet")
    {
        "tablet"
    } else if ua.contains("Mobi") || ua.contains("iPhone") || ua.contains("Android") {
        "mobile"
    } else {
        "desktop"
    }
}

pub fn browser(ua: &str) -> &'static str {
    if ua.contains("Edg/") || ua.contains("EdgA/") || ua.contains("EdgiOS/") {
        "Edge"
    } else if ua.contains("OPR/") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("YaBrowser/") {
        "Yandex"
    } else if ua.contains("SamsungBrowser/") {
        "Samsung"
    } else if ua.contains("Firefox/") || ua.contains("FxiOS/") {
        "Firefox"
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") || ua.contains("Chromium/") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "Other"
    }
}

pub fn os(ua: &str) -> &'static str {
    if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
        "iOS"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        "Other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups() {
        let edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36 Edg/130.0";
        assert_eq!(
            (device(edge), browser(edge), os(edge)),
            ("desktop", "Edge", "Windows")
        );
        let phone = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Mobile Safari/537.36";
        assert_eq!(
            (device(phone), browser(phone), os(phone)),
            ("mobile", "Chrome", "Android")
        );
        let ipad = "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
        assert_eq!(
            (device(ipad), browser(ipad), os(ipad)),
            ("tablet", "Safari", "iOS")
        );
        assert!(is_robot("Mozilla/5.0 (compatible; Googlebot/2.1)"));
        assert!(is_robot(""));
        assert!(!is_robot(edge));
    }
}
