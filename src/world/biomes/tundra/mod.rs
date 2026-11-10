use super::Climate;
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    climate.temperature < 0.24 && climate.height > 56
}

pub(super) fn surface_block(depth: i32, blocks: TerrainBlockPalette) -> u8 {
    if depth == 0 {
        blocks.grass
    } else if depth <= 4 {
        blocks.dirt
    } else {
        blocks.stone
    }
}
