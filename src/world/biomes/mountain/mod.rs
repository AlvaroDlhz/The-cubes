use super::{climate_weights, Climate};
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    let (mountain, _, _) = climate_weights(climate);
    climate.height > 82 && mountain > 0.42
}

pub(super) fn surface_block(depth: i32, blocks: TerrainBlockPalette) -> u8 {
    if depth == 0 || depth > 3 {
        blocks.stone
    } else {
        blocks.dirt
    }
}

pub(super) fn spawns_boulder(hash: u64) -> bool {
    hash % 100 < 32
}
