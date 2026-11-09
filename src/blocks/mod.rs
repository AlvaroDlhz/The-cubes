// Block types and constants
pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

// Block IDs
pub const AIR: u8 = 0;
pub const GRASS: u8 = 1;
pub const DIRT: u8 = 2;
pub const STONE: u8 = 3;
pub const SAND: u8 = 4;
pub const WOOD: u8 = 5;
pub const LEAVES: u8 = 6;
pub const APPLE: u8 = 7;
pub const WHEAT: u8 = 8;
pub const MUSHROOM: u8 = 9;
pub const WOODEN_DOOR: u8 = 10;
pub const STAIRS: u8 = 11;
pub const SLAB: u8 = 12;
pub const BRICKS: u8 = 13;
pub const PLANKS: u8 = 14;
pub const TORCH: u8 = 15;
pub const ICE: u8 = 16;
pub const SNOW: u8 = 17;
pub const OBSIDIAN: u8 = 18;
pub const LAVA: u8 = 19;
pub const MUD: u8 = 20;
pub const GRAVEL: u8 = 21;
pub const BLOCK_COUNT: usize = GRAVEL as usize + 1;
pub const LEGACY_BLOCK_COUNT: usize = 4;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodType {
    BerryRation,
    Apple,
    Mushroom,
    Wheat,
    RawMeat,
    CookedMeat,
}

impl FoodType {
    pub fn saturation(&self) -> f32 {
        match self {
            FoodType::BerryRation => 4.0,
            FoodType::Apple => 6.0,
            FoodType::Mushroom => 2.0,
            FoodType::Wheat => 8.0,
            FoodType::RawMeat => 3.0,
            FoodType::CookedMeat => 8.0,
        }
    }

    pub fn is_poisonous(&self) -> bool {
        matches!(self, FoodType::Mushroom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StairRotation {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StairHalf {
    Bottom,
    Top,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftRecipe {
    Shovel,
    Sword,
    Arrows,
    WoodenDoor,
    Stairs,
    Slab,
    Bricks,
    Planks,
    Torch,
}

#[derive(Component, Clone)]
pub struct CraftedTools {
    pub shovel_durability: u32,
    pub sword_durability: u32,
}

impl Default for CraftedTools {
    fn default() -> Self {
        Self {
            shovel_durability: 64,
            sword_durability: 64,
        }
    }
}

pub fn block_type_name(block: u8) -> &'static str {
    match block {
        AIR => "Air",
        GRASS => "Grass",
        DIRT => "Dirt",
        STONE => "Stone",
        SAND => "Sand",
        WOOD => "Wood",
        LEAVES => "Leaves",
        APPLE => "Apple",
        WHEAT => "Wheat",
        MUSHROOM => "Mushroom",
        WOODEN_DOOR => "Wooden Door",
        STAIRS => "Stairs",
        SLAB => "Slab",
        BRICKS => "Bricks",
        PLANKS => "Planks",
        TORCH => "Torch",
        ICE => "Ice",
        SNOW => "Snow",
        OBSIDIAN => "Obsidian",
        LAVA => "Lava",
        MUD => "Mud",
        GRAVEL => "Gravel",
        _ => "Unknown",
    }
}

pub fn is_block_solid(block_type: u8, door_states: &std::collections::HashMap<IVec3, DoorState>, pos: IVec3) -> bool {
    match block_type {
        AIR | LAVA => false,
        WOODEN_DOOR => {
            if let Some(&state) = door_states.get(&pos) {
                !matches!(state, DoorState::Open)
            } else {
                true
            }
        }
        STAIRS | SLAB => false,
        _ => true,
    }
}
