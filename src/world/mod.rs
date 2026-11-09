// World data and structures
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub mod terrain;
pub mod save;
pub mod collision;

pub use terrain::{generate_chunk, biome_at, terrain_height, coordinate_hash, Biome};
pub use save::{WorldSave, ChunkSave, snapshot_world, load_world_save, save_world, SAVE_VERSION};
pub use collision::{block_at, set_block, collides_with_world, mob_ground_height, light_level_at};

#[derive(Resource, Default)]
pub struct DayNightCycle {
    pub elapsed_seconds: f32,
}

impl DayNightCycle {
    pub fn daylight(&self) -> f32 {
        const GAME_DAY_SECONDS: f32 = 24.0 * 60.0 * 60.0;
        let normalized = (self.elapsed_seconds % GAME_DAY_SECONDS) / GAME_DAY_SECONDS;
        let half = (normalized * 2.0 * std::f32::consts::PI).sin();
        half * 0.5 + 0.5
    }
}

#[derive(Resource, Default)]
pub struct LoadingProgress {
    pub requested: u32,
    pub completed: u32,
}

impl LoadingProgress {
    pub fn is_complete(&self) -> bool {
        self.completed >= self.requested
    }

    pub fn progress(&self) -> f32 {
        if self.requested == 0 {
            1.0
        } else {
            (self.completed as f32) / (self.requested as f32)
        }
    }
}

#[derive(Default, PartialEq, Eq)]
pub enum LoadingStage {
    #[default]
    Inactive,
    Active,
    Complete,
}

#[derive(Resource, Default)]
pub struct WorldData {
    pub seed: u64,
    pub chunks: HashMap<IVec3, Chunk>,
    pub modified: HashSet<IVec3>,
    pub door_states: HashMap<IVec3, crate::blocks::DoorState>,
    pub player_position: Vec3,
    pub player_velocity: Vec3,
    pub name: String,
}

#[derive(Resource, Default)]
pub struct WorldStorage {
    pub saves_path: PathBuf,
    pub path: PathBuf,
    pub name: String,
    pub exists: bool,
}

#[derive(Clone, Default)]
pub struct Chunk {
    pub blocks: Vec<u8>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![0; crate::blocks::CHUNK_VOLUME],
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if !(0..crate::blocks::CHUNK_SIZE).contains(&y)
            || !(0..crate::blocks::CHUNK_SIZE).contains(&x)
            || !(0..crate::blocks::CHUNK_SIZE).contains(&z)
        {
            0
        } else {
            let idx = (y as usize) * 256 + (z as usize) * 16 + (x as usize);
            self.blocks[idx]
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: u8) {
        if (0..crate::blocks::CHUNK_SIZE).contains(&y)
            && (0..crate::blocks::CHUNK_SIZE).contains(&x)
            && (0..crate::blocks::CHUNK_SIZE).contains(&z)
        {
            let idx = (y as usize) * 256 + (z as usize) * 16 + (x as usize);
            self.blocks[idx] = block;
        }
    }
}

#[derive(Component)]
pub struct ChunkMesh(pub IVec3);
