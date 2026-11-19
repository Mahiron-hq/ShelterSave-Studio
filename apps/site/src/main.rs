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

//! ShelterSave-Studio website: `serve` the site, `watch` it, print `stats`.

mod clock;
mod config;
mod report;
mod serve;
mod stats;
mod telegram;
mod ua;
mod watch;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "shelter-site 1.0.0

Usage:
  shelter-site serve --config <file>
  shelter-site watch --config <file>
  shelter-site stats --config <file> [--from YYYY-MM-DD] [--to YYYY-MM-DD]
                     [--format text|csv|json] [--out <file>]
  shelter-site check-config --config <file>
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().map(String::as_str) else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    let opt = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if matches!(cmd, "-h" | "--help" | "help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if matches!(cmd, "-V" | "--version") {
        println!("shelter-site {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let path =
        PathBuf::from(opt("--config").unwrap_or_else(|| "/etc/shelter-site/config.toml".into()));
    let cfg = match config::Config::load(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let result = match cmd {
        "serve" => tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| e.to_string())
            .and_then(|rt| rt.block_on(serve::run(cfg))),
        "watch" => watch::run(cfg),
        "check-config" => {
            println!("config is valid");
            Ok(())
        }
        "stats" => {
            let parse = |name: &str| -> Result<Option<i64>, String> {
                opt(name)
                    .map(|v| clock::parse_date(&v).ok_or(format!("bad date {v}")))
                    .transpose()
            };
            let o = (|| -> Result<report::Options, String> {
                Ok(report::Options {
                    from: parse("--from")?,
                    to: parse("--to")?,
                    format: opt("--format").unwrap_or_else(|| "text".into()),
                })
            })();
            o.and_then(|o| report::run(&cfg, &o))
                .and_then(|text| match opt("--out") {
                    Some(file) => {
                        std::fs::write(&file, text).map_err(|e| format!("cannot write {file}: {e}"))
                    }
                    None => {
                        print!("{text}");
                        Ok(())
                    }
                })
        }
        _ => Err(format!("unknown command {cmd}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
