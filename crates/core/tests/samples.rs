// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
// SPDX-License-Identifier: GPL-3.0-or-later

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

#[test]
fn room_removal_keeps_real_saves_valid() {
    use shelter_core::actions::{run, Action, Ctx};
    use shelter_core::integrity::{check, Severity};
    use shelter_core::rules::Mode;
    use shelter_core::vault::{arr, room_ids, DWELLERS};
    let Ok(dir) = std::env::var("SHELTER_SAMPLES") else {
        return;
    };
    for entry in std::fs::read_dir(dir).expect("samples dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let root = sav::read_sav(&std::fs::read(&path).expect("read")).expect("parse");
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &root,
        };
        let dwellers = arr(&root, &DWELLERS).len();
        let mut removed = 0;
        for id in room_ids(&root) {
            let mut copy = root.clone();
            if run(&mut copy, &ctx, &Action::RemoveRoom { room: id }).is_err() {
                continue;
            }
            removed += 1;
            assert_eq!(arr(&copy, &DWELLERS).len(), dwellers);
            let errors: Vec<_> = check(&copy)
                .into_iter()
                .filter(|i| i.severity == Severity::Error)
                .collect();
            assert!(
                errors.is_empty(),
                "{} room {id}: {errors:?}",
                path.display()
            );
            assert!(sav::read_sav(sav::write_sav(&copy).as_bytes()).is_ok());
            if removed >= 8 {
                break;
            }
        }
        assert!(removed > 0, "{}", path.display());
    }
}

#[test]
fn dwellers_move_between_real_saves() {
    use shelter_core::actions::{run, Action, Ctx};
    use shelter_core::integrity::{check, Severity};
    use shelter_core::rules::Mode;
    use shelter_core::vault::{arr, DWELLERS};
    let Ok(dir) = std::env::var("SHELTER_SAMPLES") else {
        return;
    };
    let roots: Vec<_> = std::fs::read_dir(dir)
        .expect("samples dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            (path.extension()?.to_str()? == "sav").then_some(path)
        })
        .map(|p| sav::read_sav(&std::fs::read(&p).expect("read")).expect("parse"))
        .collect();
    for (a, from) in roots.iter().enumerate() {
        let target = &roots[(a + 1) % roots.len()];
        let mut copy = target.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: target,
        };
        let before = arr(&copy, &DWELLERS).len();
        let mut moved = 0;
        for d in arr(from, &DWELLERS).iter().take(10) {
            match run(
                &mut copy,
                &ctx,
                &Action::ImportDweller {
                    json: d.to_compact(),
                },
            ) {
                Ok(_) => moved += 1,
                Err(e) => assert!(
                    matches!(
                        e,
                        shelter_core::actions::ActionError::Rule(
                            shelter_core::rules::RuleError::TooManyDwellers
                        )
                    ),
                    "{e:?}"
                ),
            }
        }
        // A full vault takes nobody.
        assert!(moved > 0 || before >= 200);
        assert_eq!(arr(&copy, &DWELLERS).len(), before + moved);
        let errors: Vec<_> = check(&copy)
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(sav::read_sav(sav::write_sav(&copy).as_bytes()).is_ok());
    }
}

#[test]
fn every_field_of_real_saves_is_described() {
    use shelter_core::fields::describe;
    use shelter_core::i18n::Lang;
    use shelter_core::json::{Seg, Value};
    fn walk(v: &Value, path: &mut Vec<Seg>, missing: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                for (k, child) in &o.0 {
                    path.push(Seg::Key(k.text()));
                    for lang in [Lang::En, Lang::Ru] {
                        let f = describe(lang, path);
                        if !f.known || f.help.is_empty() {
                            missing.insert(shelter_core::json::path_to_string(path));
                        }
                    }
                    walk(child, path, missing);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, child) in a.iter().enumerate() {
                    path.push(Seg::Index(i));
                    walk(child, path, missing);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let Ok(dir) = std::env::var("SHELTER_SAMPLES") else {
        return;
    };
    let mut missing = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(dir).expect("samples dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let root = sav::read_sav(&std::fs::read(&path).expect("read")).expect("parse");
        walk(&root, &mut Vec::new(), &mut missing);
    }
    let sample: Vec<_> = missing.iter().take(40).collect();
    assert!(
        missing.is_empty(),
        "{} undescribed: {sample:?}",
        missing.len()
    );
}
