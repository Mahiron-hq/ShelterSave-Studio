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

//! .NET ticks (100 ns since 0001-01-01) to calendar date and back.
//! The game stores local time without a zone, so no conversion is done.

const TICKS_PER_SECOND: i64 = 10_000_000;
const DAYS_0001_TO_1970: i64 = 719_162;

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD HH:MM:SS`; None for values outside years 1..=9999.
pub fn format(ticks: i64) -> Option<String> {
    if !(0..=3_155_378_975_999_999_999).contains(&ticks) {
        return None;
    }
    let secs = ticks / TICKS_PER_SECOND;
    let days = secs.div_euclid(86_400) - DAYS_0001_TO_1970;
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    Some(format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    ))
}

/// Parses `YYYY-MM-DD HH:MM[:SS]`. `keep_sub` keeps the sub-second part of
/// the previous value so an untouched seconds field stays exact.
pub fn parse(text: &str, keep_sub: i64) -> Option<i64> {
    let t = text.trim().replace('T', " ");
    let (date, time) = t.split_once(' ').unwrap_or((&t, "00:00:00"));
    let dp: Vec<i64> = date
        .split('-')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    let tp: Vec<i64> = time
        .split(':')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    if dp.len() != 3 || !(2..=3).contains(&tp.len()) {
        return None;
    }
    let (y, m, d) = (dp[0], dp[1], dp[2]);
    let (hh, mm, ss) = (tp[0], tp[1], *tp.get(2).unwrap_or(&0));
    let dim = [
        31,
        if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) || d < 1 || d > dim[(m - 1) as usize] {
        return None;
    }
    if !(0..24).contains(&hh) || !(0..60).contains(&mm) || !(0..60).contains(&ss) {
        return None;
    }
    let days = days_from_civil(y, m, d) + DAYS_0001_TO_1970;
    let secs = days * 86_400 + hh * 3600 + mm * 60 + ss;
    Some(secs * TICKS_PER_SECOND + keep_sub.rem_euclid(TICKS_PER_SECOND))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = 636_820_115_098_349_470;
        let s = format(t).unwrap();
        assert_eq!(s, "2019-01-02 07:38:29");
        assert_eq!(parse(&s, t), Some(t));
        assert_eq!(format(0).unwrap(), "0001-01-01 00:00:00");
        assert!(parse("2019-02-30 00:00:00", 0).is_none());
    }
}
