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

//! Installing, updating and removing the program for the current user.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

pub const EXE: &str = "shelter-save-studio.exe";
const UNINSTALLER: &str = "uninstall.exe";
const LICENSE_FILE: &str = "LICENSE.txt";
const UNINSTALL_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ShelterSave-Studio";
const PROG_ID: &str = "ShelterSaveStudio.sav";
const CLASSES: &str = r"Software\Classes";
const NAME: &str = "ShelterSave-Studio";

pub struct Installed {
    pub dir: PathBuf,
    pub version: String,
}

pub struct Options {
    pub dir: PathBuf,
    pub start_menu: bool,
    pub desktop: bool,
    pub assoc: bool,
}

pub enum Step {
    Files,
    Shortcuts,
    Register,
}

pub fn installed() -> Option<Installed> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(UNINSTALL_KEY)
        .ok()?;
    let dir: String = key.get_value("InstallLocation").ok()?;
    let version: String = key.get_value("DisplayVersion").unwrap_or_default();
    Some(Installed {
        dir: PathBuf::from(dir),
        version,
    })
}

pub fn default_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(r"C:\"))
        .join("Programs")
        .join(NAME)
}

/// The program in `dir` cannot be replaced while it runs.
pub fn running(dir: &Path) -> bool {
    let exe = dir.join(EXE);
    exe.exists()
        && std::fs::OpenOptions::new()
            .write(true)
            .open(&exe)
            .is_err_and(|e| e.kind() != ErrorKind::NotFound)
}

fn start_menu_link() -> Option<PathBuf> {
    dirs::data_dir().map(|d| {
        d.join(r"Microsoft\Windows\Start Menu\Programs")
            .join(format!("{NAME}.lnk"))
    })
}

fn desktop_link() -> Option<PathBuf> {
    dirs::desktop_dir().map(|d| d.join(format!("{NAME}.lnk")))
}

/// Writes `data` next to `path` and moves it over, so a failed write leaves the old file.
fn replace(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("new");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn link(target: &Path, dir: &Path, lnk: &Path) -> io::Result<()> {
    let err = |e: mslnk::MSLinkError| io::Error::other(e.to_string());
    let mut sl = mslnk::ShellLink::new(target).map_err(err)?;
    sl.set_working_dir(Some(dir.display().to_string()));
    sl.set_icon_location(Some(target.display().to_string()));
    sl.create_lnk(lnk).map_err(err)
}

pub fn install(
    opt: &Options,
    payload: &[u8],
    license: &str,
    version: &str,
    mut progress: impl FnMut(Step),
) -> io::Result<()> {
    progress(Step::Files);
    std::fs::create_dir_all(&opt.dir)?;
    let exe = opt.dir.join(EXE);
    replace(&exe, payload)?;
    let me = std::env::current_exe()?;
    let uninstaller = opt.dir.join(UNINSTALLER);
    if me != uninstaller {
        replace(&uninstaller, &std::fs::read(&me)?)?;
    }
    replace(&opt.dir.join(LICENSE_FILE), license.as_bytes())?;

    progress(Step::Shortcuts);
    for (wanted, place) in [
        (opt.start_menu, start_menu_link()),
        (opt.desktop, desktop_link()),
    ] {
        let Some(place) = place else { continue };
        if wanted {
            link(&exe, &opt.dir, &place)?;
        } else if place.exists() {
            std::fs::remove_file(&place)?;
        }
    }

    progress(Step::Register);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(UNINSTALL_KEY)?;
    let dir = opt.dir.display().to_string();
    let size_kb = u32::try_from(payload.len() / 1024).unwrap_or(u32::MAX);
    key.set_value("DisplayName", &NAME)?;
    key.set_value("DisplayVersion", &version)?;
    key.set_value("Publisher", &"Lev Burmistrov (Mahiron-hq)")?;
    key.set_value("InstallLocation", &dir)?;
    key.set_value("DisplayIcon", &exe.display().to_string())?;
    key.set_value(
        "UninstallString",
        &format!("\"{}\" --uninstall", uninstaller.display()),
    )?;
    key.set_value(
        "URLInfoAbout",
        &"https://github.com/Mahiron-hq/ShelterSave-Studio",
    )?;
    key.set_value("EstimatedSize", &size_kb)?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    if opt.assoc {
        associate(&exe)?;
    } else {
        unassociate();
    }
    Ok(())
}

fn associate(exe: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (prog, _) = hkcu.create_subkey(format!(r"{CLASSES}\{PROG_ID}"))?;
    prog.set_value("", &"Fallout Shelter save")?;
    let (icon, _) = prog.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{}\",0", exe.display()))?;
    let (cmd, _) = prog.create_subkey(r"shell\open\command")?;
    cmd.set_value("", &format!("\"{}\" \"%1\"", exe.display()))?;
    let (ext, _) = hkcu.create_subkey(format!(r"{CLASSES}\.sav"))?;
    let (with, _) = ext.create_subkey("OpenWithProgids")?;
    with.set_value(PROG_ID, &"")?;
    // Only take the default when no other program has it.
    let current: String = ext.get_value("").unwrap_or_default();
    if current.is_empty() {
        ext.set_value("", &PROG_ID)?;
    }
    notify_assoc();
    Ok(())
}

fn unassociate() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(format!(r"{CLASSES}\{PROG_ID}"));
    if let Ok(ext) = hkcu.open_subkey_with_flags(
        format!(r"{CLASSES}\.sav"),
        winreg::enums::KEY_READ | winreg::enums::KEY_WRITE,
    ) {
        if let Ok(with) = ext.open_subkey_with_flags("OpenWithProgids", winreg::enums::KEY_WRITE) {
            let _ = with.delete_value(PROG_ID);
        }
        if ext.get_value::<String, _>("").is_ok_and(|v| v == PROG_ID) {
            let _ = ext.delete_value("");
        }
    }
    notify_assoc();
}

fn notify_assoc() {
    use windows_sys::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    // SAFETY: a plain notification without item pointers.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

/// Settings, recent files, recovery data and backups in the default folders.
fn data_dirs() -> Vec<PathBuf> {
    [dirs::config_dir(), dirs::data_local_dir()]
        .into_iter()
        .flatten()
        .map(|d| d.join(NAME))
        .collect()
}

pub fn uninstall(dir: &Path, remove_data: bool) -> io::Result<()> {
    for place in [start_menu_link(), desktop_link()].into_iter().flatten() {
        if place.exists() {
            std::fs::remove_file(&place)?;
        }
    }
    unassociate();
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(UNINSTALL_KEY);
    for f in [EXE, LICENSE_FILE] {
        let p = dir.join(f);
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }
    if remove_data {
        for d in data_dirs() {
            if d.is_dir() {
                std::fs::remove_dir_all(&d)?;
            }
        }
    }
    // The running uninstaller cannot delete itself: a short-lived shell does it after exit.
    let me = std::env::current_exe()?;
    if me.parent() == Some(dir) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let script = format!(
            "ping 127.0.0.1 -n 3 > nul & del /f /q \"{}\" & rmdir \"{}\"",
            me.display(),
            dir.display()
        );
        // cmd reads the raw command line, so the script must not be re-quoted.
        std::process::Command::new("cmd")
            .args(["/d", "/c"])
            .raw_arg(&script)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    } else {
        let _ = std::fs::remove_dir(dir);
    }
    Ok(())
}

/// Starts the installed program without waiting for it.
pub fn launch(dir: &Path) {
    let _ = std::process::Command::new(dir.join(EXE))
        .current_dir(dir)
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Writes to the real per-user registry, so it runs only on request.
    #[test]
    fn install_update_remove() {
        if std::env::var_os("SHELTER_INSTALL_TEST").is_none() || installed().is_some() {
            return;
        }
        let dir = std::env::temp_dir().join("shelter-setup-test");
        let _ = std::fs::remove_dir_all(&dir);
        let opt = Options {
            dir: dir.clone(),
            start_menu: false,
            desktop: false,
            assoc: false,
        };
        install(&opt, b"first", "license", "1.0.0", |_| {}).unwrap();
        assert_eq!(std::fs::read(dir.join(EXE)).unwrap(), b"first");
        let found = installed().unwrap();
        assert_eq!(found.dir, dir);
        assert_eq!(found.version, "1.0.0");
        assert!(!running(&dir));
        install(&opt, b"second", "license", "1.0.0", |_| {}).unwrap();
        assert_eq!(std::fs::read(dir.join(EXE)).unwrap(), b"second");
        assert!(dir.join(UNINSTALLER).exists());
        uninstall(&dir, false).unwrap();
        assert!(installed().is_none());
        assert!(!dir.join(EXE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
