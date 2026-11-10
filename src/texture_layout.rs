pub const ATLAS_COLUMNS: u32 = 16;
pub const TILE_SIZE: u32 = 32;
pub const ATLAS_SIZE: u32 = ATLAS_COLUMNS * TILE_SIZE;

include!(concat!(env!("OUT_DIR"), "/texture_layout_generated.rs"));
