//! Reads the user's installed game when `SHELTER_GAME` points to its folder.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use shelter_art::bundle::{Bundle, Source};
use shelter_art::dweller::{self, Look, Mesh, Parts};
use shelter_art::image;

struct FileSource(std::path::PathBuf);

impl Source for FileSource {
    fn read_at(&self, at: u64, len: usize) -> Option<Vec<u8>> {
        let mut f = std::fs::File::open(&self.0).ok()?;
        f.seek(SeekFrom::Start(at)).ok()?;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf).ok()?;
        Some(buf)
    }
}

#[test]
fn pictures_from_the_game() {
    let Ok(dir) = std::env::var("SHELTER_GAME") else {
        return;
    };
    let path = std::path::Path::new(&dir)
        .join("FalloutShelter_Data")
        .join(shelter_art::bundle_name().unwrap());
    let bundle = Bundle::open(FileSource(path)).expect("bundle opens");
    let read = |name: &str| {
        let (node, off, size) = image::texture_place(name)?;
        image::texture(name, bundle.read(&node, off, size)?)
    };
    let atlases = shelter_art::icons_by_atlas();
    let (tex, list) = &atlases[0];
    let atlas = read(tex).expect("atlas decodes");
    let (key, rect) = &list[0];
    assert!(image::cut_icon(&atlas, *rect, image::max_side(key)).is_some());

    let (node, v, i) = dweller::mesh_place("female").unwrap();
    let mesh = Mesh::new(
        "female",
        &bundle.read(&node, v.0, v.1).unwrap(),
        &bundle.read(&node, i.0, i.1).unwrap(),
    )
    .expect("mesh decodes");
    let look = Look {
        male: false,
        skin: [1.0, 0.9, 0.8],
        hair_color: [0.8, 0.6, 0.2],
        hair: None,
        face_mask: None,
        face: "Smile".into(),
        outfit: None,
        outfit_color: [1.0; 3],
        helmet: None,
        helmet_cover: None,
    };
    let mut images = HashMap::new();
    let missing = match Parts::new(&images, &look) {
        Err(m) => m,
        Ok(_) => Vec::new(),
    };
    for name in missing {
        images.insert(
            name.clone(),
            Arc::new(read(&name).expect("texture decodes")),
        );
    }
    let Ok(parts) = Parts::new(&images, &look) else {
        panic!("all parts loaded")
    };
    let pic = dweller::render(&parts, &mesh).expect("dweller renders");
    assert!(pic.w > 50 && pic.h > 200);
}

#[test]
fn journal_texts_from_the_game() {
    let Ok(dir) = std::env::var("SHELTER_GAME") else {
        return;
    };
    let path = std::path::Path::new(&dir)
        .join("FalloutShelter_Data")
        .join(shelter_art::bundle_name().unwrap());
    let bundle = Bundle::open(FileSource(path)).expect("bundle opens");
    assert!(
        shelter_art::load_texts(&bundle),
        "phrases match the catalog"
    );
    let texts = shelter_core::catalog::game_texts().unwrap();
    assert_eq!(texts.texts["0"].en, "Got {0} points of radiation damage.");
    assert_eq!(texts.texts["0"].ru, "{0} ед. урона от радиации.");
    assert_eq!(texts.keys.len(), 20);
}
