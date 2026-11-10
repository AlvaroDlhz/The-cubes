use crate::world::terrain::TerrainBlockPalette;

// Keep these values here so the pradera can be tuned without changing the
// forest or the shared chunk generator.
const GRASS_PATCH_THRESHOLD: f32 = 0.55;
const FLOWER_PATCH_THRESHOLD: f32 = 0.66;
const DENSE_PATCH_THRESHOLD: f32 = 0.84;
const TREE_GROVE_THRESHOLD: f32 = 0.74;
const TREE_CHANCE: u64 = 1;

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
    // The same broad noise creates clearings, grassy patches, and flower beds.
    // Wheat remains a scarce resource inside the densest patches.
    let chance = roll % 100;
    if patch > DENSE_PATCH_THRESHOLD && chance < 1 {
        blocks.wheat
    } else if patch > FLOWER_PATCH_THRESHOLD && chance < 13 {
        if roll % 2 == 0 {
            blocks.red_flower
        } else {
            blocks.yellow_flower
        }
    } else if patch > GRASS_PATCH_THRESHOLD && chance < 48 {
        blocks.tall_grass
    } else {
        blocks.air
    }
}

pub(super) fn spawns_tree(grove: f32, roll: u64) -> bool {
    grove > TREE_GROVE_THRESHOLD && roll % 100 < TREE_CHANCE
}

pub(super) fn tree_height(hash: u64) -> i32 {
    4 + (hash % 3) as i32
}

pub(super) fn spawns_cabin() -> bool {
    true
}
