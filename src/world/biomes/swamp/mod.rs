use super::{climate_weights, Climate};
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    let (_, _, swamp) = climate_weights(climate);
    swamp > 0.38
}

pub(super) fn surface_block(depth: i32, blocks: TerrainBlockPalette) -> u8 {
    if depth == 0 {
        blocks.dirt
    } else if depth <= 4 {
        blocks.dirt
    } else {
        blocks.stone
    }
}

pub(super) fn plant_block(patch: f32, roll: u64, blocks: TerrainBlockPalette) -> u8 {
    if patch > 0.52 && roll < 9 {
        blocks.mushroom
    } else {
        blocks.air
    }
}
