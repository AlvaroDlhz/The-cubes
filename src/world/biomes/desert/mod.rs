use super::{climate_weights, Climate};
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    let (_, desert, _) = climate_weights(climate);
    desert > 0.40
}

pub(super) fn surface_block(depth: i32, blocks: TerrainBlockPalette) -> u8 {
    if depth <= 5 {
        blocks.sand
    } else {
        blocks.stone
    }
}
