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

//! Disk access: atomic writes, backups, save discovery.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use shelter_core::sav;
use shelter_core::vault;

/// Writes to a temporary file next to the target, then renames it over the
/// target, so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Size and modification time, used to notice changes made by the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    pub modified: Option<SystemTime>,
}

pub fn stamp(path: &Path) -> Option<Stamp> {
    let m = fs::metadata(path).ok()?;
    Some(Stamp {
        len: m.len(),
        modified: m.modified().ok(),
    })
}

fn backup_folder(root: &Path, file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "save".into());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in file.to_string_lossy().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    root.join(format!("{stem}-{:08x}", h as u32))
}

/// Copies `file` into the backup folder and keeps at most `keep` copies.
pub fn make_backup(root: &Path, file: &Path, keep: u32) -> std::io::Result<PathBuf> {
    let dir = backup_folder(root, file);
    fs::create_dir_all(&dir)?;
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| "sav".into());
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let mut target = dir.join(format!("{stamp}.{ext}"));
    let mut n = 1;
    while target.exists() {
        target = dir.join(format!("{stamp}_{n}.{ext}"));
        n += 1;
    }
    let data = fs::read(file)?;
    write_atomic(&target, &data)?;
    let mut all = list_backups(root, file);
    while all.len() > keep as usize {
        if let Some(old) = all.pop() {
            let _ = fs::remove_file(old.path);
        }
    }
    Ok(target)
}

pub struct Backup {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
}

/// Backups of a file, newest first.
pub fn list_backups(root: &Path, file: &Path) -> Vec<Backup> {
    let dir = backup_folder(root, file);
    let mut out: Vec<Backup> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| Backup {
            modified: e.metadata().ok().and_then(|m| m.modified().ok()),
            path: e.path(),
        })
        .collect();
    out.sort_by(|a, b| b.path.file_name().cmp(&a.path.file_name()));
    out
}

/// Standard save folders of the PC editions (Steam and Bethesda.net).
pub fn standard_save_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("FalloutShelter"));
        }
    } else if let Some(home) = dirs::home_dir() {
        for steam in [
            ".steam/steam",
            ".local/share/Steam",
            ".var/app/com.valvesoftware.Steam/data/Steam",
        ] {
            out.push(
                home.join(steam)
                    .join("steamapps/compatdata/588430/pfx/drive_c/users/steamuser/AppData/Local/FalloutShelter"),
            );
        }
    }
    out
}

pub struct FoundSave {
    pub path: PathBuf,
    pub vault_name: String,
    pub dwellers: usize,
    pub modified: Option<SystemTime>,
}

const MAX_SCAN: usize = 64;

/// Lists readable vault saves in the given folders (not recursive).
pub fn discover(extra: &[PathBuf]) -> Vec<FoundSave> {
    let mut out = Vec::new();
    for dir in standard_save_dirs().iter().chain(extra.iter()) {
        let Ok(rd) = fs::read_dir(dir) else { continue };
        for e in rd.flatten().take(MAX_SCAN) {
            let path = e.path();
            let is_sav = path
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("sav"));
            if !is_sav || out.iter().any(|f: &FoundSave| f.path == path) {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else { continue };
            let Ok(root) = sav::read_sav(&bytes) else {
                continue;
            };
            out.push(FoundSave {
                vault_name: vault::text(&root, &["vault", "VaultName"]).unwrap_or_default(),
                dwellers: vault::arr(&root, &vault::DWELLERS).len(),
                modified: e.metadata().ok().and_then(|m| m.modified().ok()),
                path,
            });
        }
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.modified));
    out
}

pub fn format_time(t: Option<SystemTime>) -> String {
    t.map(|t| {
        chrono::DateTime::<chrono::Local>::from(t)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_and_backups() {
        let dir = std::env::temp_dir().join(format!("sss-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Vault1.sav");
        write_atomic(&file, b"one").unwrap();
        write_atomic(&file, b"two").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"two");
        let root = dir.join("backups");
        for _ in 0..4 {
            make_backup(&root, &file, 2).unwrap();
        }
        assert_eq!(list_backups(&root, &file).len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}

/// Unsaved edits kept on disk, so they survive a crash of the editor.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Recovery {
    pub path: Option<PathBuf>,
    pub session: shelter_core::document::Session,
}

fn recovery_file() -> Option<PathBuf> {
    crate::settings::Settings::dir().map(|d| d.join("recovery.json"))
}

pub fn write_recovery(data: &str) -> std::io::Result<()> {
    let Some(path) = recovery_file() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    write_atomic(&path, data.as_bytes())
}

pub fn read_recovery() -> Option<Recovery> {
    let text = fs::read_to_string(recovery_file()?).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn clear_recovery() {
    if let Some(path) = recovery_file() {
        let _ = fs::remove_file(path);
    }
}
