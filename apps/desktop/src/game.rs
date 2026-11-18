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

//! Watches whether Fallout Shelter is running: the game writes its own
//! state over the save while it runs, so edits made then are lost.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;

pub fn watch(ctx: egui::Context) -> Arc<AtomicBool> {
    // The first check runs in the thread too: starting `tasklist` takes a while.
    let flag = Arc::new(AtomicBool::new(false));
    let out = flag.clone();
    let spawned = std::thread::Builder::new()
        .name("game-watch".into())
        .spawn(move || loop {
            let now = is_running();
            if flag.swap(now, Ordering::Relaxed) != now {
                ctx.request_repaint();
            }
            std::thread::sleep(Duration::from_secs(3));
        });
    if spawned.is_err() {
        out.store(false, Ordering::Relaxed);
    }
    out
}

#[cfg(windows)]
fn is_running() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("tasklist")
        .args([
            "/FI",
            "IMAGENAME eq FalloutShelter.exe",
            "/NH",
            "/FO",
            "CSV",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_ascii_lowercase()
                .contains("falloutshelter.exe")
        })
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn is_running() -> bool {
    // Under Proton the process name is cut to 15 bytes.
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return false;
    };
    dir.flatten().any(|e| {
        std::fs::read_to_string(e.path().join("comm"))
            .is_ok_and(|c| c.trim().to_ascii_lowercase().starts_with("falloutshelter"))
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn is_running() -> bool {
    false
}
