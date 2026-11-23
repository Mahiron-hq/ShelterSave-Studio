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

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    println!("cargo:rerun-if-env-changed=SHELTER_PAYLOAD");
    println!("cargo:rerun-if-changed=../../assets/app-icon.ico");
    // The program to install; without it the installer only shows an error.
    let payload = std::env::var_os("SHELTER_PAYLOAD").map(PathBuf::from);
    match payload {
        Some(p) => {
            println!("cargo:rerun-if-changed={}", p.display());
            std::fs::copy(&p, out.join("payload.bin")).expect("SHELTER_PAYLOAD is readable");
        }
        None => std::fs::write(out.join("payload.bin"), []).expect("write empty payload"),
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon.ico")
        .set("ProductName", "ShelterSave-Studio")
        .set("FileDescription", "ShelterSave-Studio setup")
        .set("CompanyName", "Lev Burmistrov (Mahiron-hq)")
        .set(
            "LegalCopyright",
            "Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)",
        )
        .set("OriginalFilename", "shelter-save-studio-setup.exe");
    if let Err(e) = res.compile() {
        panic!("Windows resources (icon, version) could not be embedded: {e}");
    }
}
