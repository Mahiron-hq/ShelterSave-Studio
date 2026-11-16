// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
// SPDX-License-Identifier: GPL-3.0-only

//! Round trip of real saves from the folder in SHELTER_SAMPLES, if set.

use shelter_core::sav;

#[test]
fn real_saves_round_trip_byte_for_byte() {
    let Ok(dir) = std::env::var("SHELTER_SAMPLES") else {
        eprintln!("SHELTER_SAMPLES not set, skipped");
        return;
    };
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).expect("samples dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read sample");
        let root = sav::read_sav(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let out = sav::write_sav(&root);
        assert_eq!(out.as_bytes(), bytes.trim_ascii(), "{}", path.display());
        checked += 1;
    }
    assert!(checked > 0, "no .sav files in SHELTER_SAMPLES");
}

#[test]
fn real_saves_have_no_integrity_errors() {
    let Ok(dir) = std::env::var("SHELTER_SAMPLES") else {
        return;
    };
    for entry in std::fs::read_dir(dir).expect("samples dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let root = sav::read_sav(&std::fs::read(&path).expect("read")).expect("parse");
        let issues = shelter_core::integrity::check(&root);
        for i in &issues {
            eprintln!(
                "{}: {:?} {} {:?}",
                path.display(),
                i.severity,
                i.code,
                i.params
            );
        }
        assert!(
            issues
                .iter()
                .all(|i| i.severity != shelter_core::integrity::Severity::Error),
            "{}",
            path.display()
        );
    }
}
