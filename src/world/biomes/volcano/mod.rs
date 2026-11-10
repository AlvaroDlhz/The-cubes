use super::Climate;
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    climate.height > 98 && climate.temperature > 0.68 && climate.ridges > 0.88
}

pub(super) fn surface_block(_depth: i32, blocks: TerrainBlockPalette) -> u8 {
    blocks.stone
}
