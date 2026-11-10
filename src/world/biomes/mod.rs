//! Per-biome terrain and feature rules.
//!
//! Shared noise and chunk assembly stay in `world::terrain`; biome-specific
//! materials, climate thresholds, and features live in their own modules.

use super::{Biome, TerrainBlockPalette};

mod beach;
mod cave;
mod desert;
mod forest;
mod mountain;
mod plains;
mod swamp;
mod tundra;
mod volcano;

#[derive(Clone, Copy)]
struct Climate {
    height: i32,
    temperature: f32,
    moisture: f32,
    ridges: f32,
}

pub(super) fn classify(height: i32, temperature: f32, moisture: f32, ridges: f32) -> Biome {
    let climate = Climate {
        height,
        temperature,
        moisture,
        ridges,
    };
    if beach::matches(climate) {
        Biome::Beach
    } else if volcano::matches(climate) {
        Biome::Volcano
    } else if mountain::matches(climate) {
        Biome::Mountain
    } else if tundra::matches(climate) {
        Biome::Tundra
    } else if desert::matches(climate) {
        Biome::Desert
    } else if swamp::matches(climate) {
        Biome::Swamp
    } else if forest::matches(climate) {
        Biome::Forest
    } else {
        Biome::Plains
    }
}

fn climate_weights(climate: Climate) -> (f32, f32, f32) {
    let mountain = super::smoothstep(0.58, 0.82, climate.ridges);
    let desert = super::smoothstep(0.57, 0.76, climate.temperature)
        * (1.0 - super::smoothstep(0.42, 0.62, climate.moisture));
    let swamp = super::smoothstep(0.68, 0.82, climate.moisture)
        * (1.0 - super::smoothstep(0.48, 0.64, climate.ridges));
    (mountain, desert, swamp)
}

pub(super) fn surface_block(biome: Biome, depth: i32, blocks: TerrainBlockPalette) -> u8 {
    match biome {
        Biome::Plains => plains::surface_block(depth, blocks),
        Biome::Forest => forest::surface_block(depth, blocks),
        Biome::Desert => desert::surface_block(depth, blocks),
        Biome::Mountain => mountain::surface_block(depth, blocks),
        Biome::Tundra => tundra::surface_block(depth, blocks),
        Biome::Beach => beach::surface_block(depth, blocks),
        Biome::Swamp => swamp::surface_block(depth, blocks),
        Biome::Volcano => volcano::surface_block(depth, blocks),
        Biome::Cave => cave::surface_block(depth, blocks),
    }
}

pub(super) fn plant_block(biome: Biome, patch: f32, roll: u64, blocks: TerrainBlockPalette) -> u8 {
    match biome {
        Biome::Plains => plains::plant_block(patch, roll, blocks),
        Biome::Forest => forest::plant_block(patch, roll, blocks),
        Biome::Swamp => swamp::plant_block(patch, roll, blocks),
        _ => blocks.air,
    }
}

pub(super) fn spawns_tree(biome: Biome, grove: f32, roll: u64) -> bool {
    match biome {
        Biome::Forest => forest::spawns_tree(grove, roll),
        Biome::Plains => plains::spawns_tree(grove, roll),
        _ => false,
    }
}

pub(super) fn tree_height(biome: Biome, hash: u64) -> i32 {
    match biome {
        Biome::Forest => forest::tree_height(hash),
        Biome::Plains => plains::tree_height(hash),
        _ => 0,
    }
}

pub(super) fn spawns_boulder(biome: Biome, hash: u64) -> bool {
    biome == Biome::Mountain && mountain::spawns_boulder(hash)
}

pub(super) fn spawns_ruin(biome: Biome) -> bool {
    match biome {
        Biome::Forest => forest::spawns_ruin(),
        _ => false,
    }
}

pub(super) fn spawns_cabin(biome: Biome) -> bool {
    biome == Biome::Plains && plains::spawns_cabin()
}
