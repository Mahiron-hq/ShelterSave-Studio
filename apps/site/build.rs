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
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let dist = manifest.join("../../web/dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    // Without a web build the server compiles with an empty site, for checks and tests.
    let dir = if dist.join("en").is_dir() {
        dist.canonicalize().expect("dist path")
    } else {
        let empty = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("empty-dist");
        std::fs::create_dir_all(&empty).expect("create empty dist");
        empty
    };
    println!("cargo:rustc-env=SITE_DIST={}", dir.display());
}
