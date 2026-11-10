use image::{GenericImage, ImageReader, RgbaImage};
use serde_json::Value;
use std::{env, fs, path::Path};

const ATLAS_COLUMNS: u32 = 16;
const TILE_SIZE: u32 = 32;
const ATLAS_SIZE: u32 = ATLAS_COLUMNS * TILE_SIZE;

fn main() {
    let manifest_path = Path::new("assets/blocks/textures.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed=assets/blocks");

    let manifest: Value = serde_json::from_slice(
        &fs::read(manifest_path).expect("Cannot read block texture manifest"),
    )
    .expect("Block texture manifest must be valid JSON");
    let textures = manifest["textures"]
        .as_array()
        .expect("Texture manifest must contain a textures array");
    assert!(
        textures.len() <= (ATLAS_COLUMNS * ATLAS_COLUMNS) as usize,
        "Texture manifest exceeds the {ATLAS_COLUMNS}x{ATLAS_COLUMNS} atlas capacity"
    );

    let mut atlas = RgbaImage::new(ATLAS_SIZE, ATLAS_SIZE);
    let mut generated = String::from("pub const TEXTURE_FILES: &[(usize, &str)] = &[\n");
    let mut lookup =
        String::from("pub fn texture_index(name: &str) -> usize {\n    match name {\n");
    let mut names = std::collections::HashSet::new();

    for (tile, entry) in textures.iter().enumerate() {
        let name = entry["name"].as_str().expect("Texture entry needs a name");
        let file = entry["file"].as_str().expect("Texture entry needs a file");
        assert!(names.insert(name), "Duplicate texture name: {name}");
        assert!(
            Path::new(file)
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))),
            "Texture path must stay inside assets/blocks: {file}"
        );

        let path = Path::new("assets/blocks").join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let image = ImageReader::open(&path)
            .unwrap_or_else(|error| panic!("Cannot open {}: {error}", path.display()))
            .decode()
            .unwrap_or_else(|error| panic!("Cannot decode {}: {error}", path.display()))
            .to_rgba8();
        assert_eq!(
            image.dimensions(),
            (TILE_SIZE, TILE_SIZE),
            "{} must be {TILE_SIZE}x{TILE_SIZE} pixels",
            path.display()
        );
        atlas
            .copy_from(
                &image,
                (tile as u32 % ATLAS_COLUMNS) * TILE_SIZE,
                (tile as u32 / ATLAS_COLUMNS) * TILE_SIZE,
            )
            .expect("Texture fits in the atlas");

        generated.push_str(&format!("    ({tile}, {file:?}),\n"));
        lookup.push_str(&format!("        {name:?} => {tile},\n"));
    }
    assert!(
        names.contains("stone"),
        "Texture manifest must define the `stone` fallback"
    );

    generated.push_str("];\n\n");
    lookup.push_str("        _ => panic!(\"Unknown texture alias: {}\", name),\n    }\n}\n");
    generated.push_str(&lookup);

    let output_dir = env::var_os("OUT_DIR").expect("OUT_DIR is set");
    let output_dir = Path::new(&output_dir);
    fs::write(output_dir.join("block_atlas.rgba"), atlas.as_raw())
        .expect("Cannot write generated block atlas");
    fs::write(output_dir.join("texture_layout_generated.rs"), generated)
        .expect("Cannot write generated texture lookup");
}
