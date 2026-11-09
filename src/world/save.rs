#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct WorldSave {
    pub version: u32,
    pub name: String,
    pub seed: u64,
    pub player_position: [f32; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub hunger: f32,
    pub health: f32,
    pub experience_level: u32,
    pub shovel_durability: u32,
    pub sword_durability: u32,
    pub chunks: Vec<ChunkSave>,
}

#[derive(Serialize, Deserialize)]
pub struct ChunkSave {
    pub coord: [i32; 3],
    pub blocks: Vec<u8>,
}

pub const SAVE_VERSION: u32 = 1;

pub fn snapshot_world(
    world: &crate::world::WorldData,
    player_pos: Vec3,
    player_yaw: f32,
    player_pitch: f32,
    hunger: f32,
    health: f32,
    experience_level: u32,
    shovel_durability: u32,
    sword_durability: u32,
    name: String,
) -> WorldSave {
    let mut chunks = Vec::new();
    for (coord, chunk) in &world.chunks {
        chunks.push(ChunkSave {
            coord: [coord.x, coord.y, coord.z],
            blocks: chunk.blocks.clone(),
        });
    }
    WorldSave {
        version: SAVE_VERSION,
        name,
        seed: world.seed,
        player_position: player_pos.to_array(),
        player_yaw,
        player_pitch,
        hunger,
        health,
        experience_level,
        shovel_durability,
        sword_durability,
        chunks,
    }
}

pub fn load_world_save(path: &PathBuf) -> Result<WorldSave, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let save: WorldSave = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    if save.version != SAVE_VERSION {
        return Err(format!("Unsupported save version: {}", save.version));
    }
    
    Ok(save)
}

pub fn save_world(world_save: &WorldSave, path: &PathBuf) -> Result<(), String> {
    let json = serde_json::to_string_pretty(world_save)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))
}
