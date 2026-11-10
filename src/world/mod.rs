//! World data shared by terrain generation and the game.
use bevy::prelude::*;

use crate::blocks::{AIR, CHUNK_SIZE, CHUNK_VOLUME};

pub(crate) mod data;
pub(crate) mod legacy;
pub mod terrain;

pub(crate) use data::*;
pub(crate) use legacy::*;

pub use terrain::{
    biome_at, coordinate_hash, generate_chunk, is_cave, set_generated_block, terrain_height, Biome,
};

/// A cube of `CHUNK_SIZE`³ block ids.
#[derive(Clone)]
pub struct Chunk {
    pub blocks: Vec<u8>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![AIR; CHUNK_VOLUME],
        }
    }

    /// Reads a block at in-bounds local coordinates.
    pub fn get(&self, local: IVec3) -> u8 {
        self.blocks[block_index(local)]
    }

    /// Writes a block at in-bounds local coordinates.
    pub fn set(&mut self, local: IVec3, block: u8) {
        self.blocks[block_index(local)] = block;
    }

    /// Like [`Chunk::get`], but coordinates outside the chunk read as air.
    pub fn get_xyz(&self, x: i32, y: i32, z: i32) -> u8 {
        if in_chunk(x, y, z) {
            self.get(IVec3::new(x, y, z))
        } else {
            AIR
        }
    }

    /// Like [`Chunk::set`], but coordinates outside the chunk are ignored.
    pub fn set_xyz(&mut self, x: i32, y: i32, z: i32, block: u8) {
        if in_chunk(x, y, z) {
            self.set(IVec3::new(x, y, z), block);
        }
    }
}

fn in_chunk(x: i32, y: i32, z: i32) -> bool {
    (0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y) && (0..CHUNK_SIZE).contains(&z)
}

pub fn block_index(local: IVec3) -> usize {
    (local.x + CHUNK_SIZE * (local.z + CHUNK_SIZE * local.y)) as usize
}
