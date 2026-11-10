use super::Climate;
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    (climate.height - 48).abs() <= 2
}

pub(super) fn surface_block(depth: i32, blocks: TerrainBlockPalette) -> u8 {
    if depth <= 4 {
        blocks.sand
    } else {
        blocks.stone
    }
}
