use super::{climate_weights, Climate};
use crate::world::terrain::TerrainBlockPalette;

pub(super) fn matches(climate: Climate) -> bool {
    let (_, desert, swamp) = climate_weights(climate);
    let forest = super::super::smoothstep(0.48, 0.67, climate.moisture);
    forest > 0.48 && desert <= 0.40 && swamp <= 0.38
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

pub(super) fn plant_block(patch: f32, roll: u64, blocks: TerrainBlockPalette) -> u8 {
    if patch > 0.57 && roll < 17 {
        blocks.mushroom
    } else {
        blocks.air
    }
}

pub(super) fn spawns_tree(grove: f32, roll: u64) -> bool {
    grove >= 0.43 && roll % 100 < 23
}

pub(super) fn tree_height(hash: u64) -> i32 {
    4 + (hash % 3) as i32
}

pub(super) fn spawns_ruin() -> bool {
    true
}
