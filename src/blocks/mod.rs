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
// Keep the IDs already used by saved game worlds stable.
pub const WATER: u8 = 16;
pub const LAVA: u8 = 17;
pub const ICE: u8 = 18;
pub const SNOW: u8 = 19;
pub const OBSIDIAN: u8 = 20;
pub const MUD: u8 = 21;
pub const GRAVEL: u8 = 22;
pub const RED_FLOWER: u8 = 23;
pub const YELLOW_FLOWER: u8 = 24;
pub const TALL_GRASS: u8 = 25;
pub const COAL_ORE: u8 = 26;
pub const COPPER_ORE: u8 = 27;
pub const IRON_ORE: u8 = 28;
pub const BLOCK_COUNT: usize = IRON_ORE as usize + 1;
pub const LEGACY_BLOCK_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MineralKind {
    Coal,
    RawCopper,
    RawIron,
}

#[derive(Clone, Copy, Debug)]
pub enum BlockDropDefinition {
    Block(u8),
    Mineral(MineralKind),
}

/// Shared gameplay and presentation data for a block type. World chunks still
/// store the stable numeric ID, while systems read behavior from this table.
#[derive(Clone, Copy, Debug)]
pub struct BlockDefinition {
    pub id: u8,
    pub name_en: &'static str,
    pub name_es: &'static str,
    pub texture_top: &'static str,
    pub texture_side: &'static str,
    pub texture_bottom: &'static str,
    pub solid: bool,
    pub requires_pickaxe: bool,
    pub hardness: f32,
    pub drop: Option<BlockDropDefinition>,
    pub experience: u32,
    pub drop_color: [f32; 3],
}

const fn block_def(
    id: u8,
    name_en: &'static str,
    name_es: &'static str,
    texture: &'static str,
    solid: bool,
    hardness: f32,
    drop: Option<u8>,
    experience: u32,
    drop_color: [f32; 3],
) -> BlockDefinition {
    BlockDefinition {
        id,
        name_en,
        name_es,
        texture_top: texture,
        texture_side: texture,
        texture_bottom: texture,
        solid,
        requires_pickaxe: false,
        hardness,
        drop: match drop {
            Some(block) => Some(BlockDropDefinition::Block(block)),
            None => None,
        },
        experience,
        drop_color,
    }
}

const BLOCKS: [BlockDefinition; BLOCK_COUNT] = [
    block_def(
        AIR,
        "Air",
        "Vacío",
        "stone",
        false,
        0.0,
        None,
        0,
        [0.7, 0.7, 0.7],
    ),
    BlockDefinition {
        texture_top: "grass_top",
        texture_side: "grass_side",
        texture_bottom: "dirt",
        ..block_def(
            GRASS,
            "Grass",
            "Césped",
            "grass_side",
            true,
            0.8,
            Some(GRASS),
            1,
            [0.3, 0.7, 0.3],
        )
    },
    block_def(
        DIRT,
        "Dirt",
        "Tierra",
        "dirt",
        true,
        0.8,
        Some(DIRT),
        1,
        [0.6, 0.4, 0.2],
    ),
    block_def(
        STONE,
        "Stone",
        "Piedra",
        "stone",
        true,
        3.0,
        Some(STONE),
        2,
        [0.5, 0.5, 0.5],
    ),
    block_def(
        SAND,
        "Sand",
        "Arena",
        "sand",
        true,
        0.8,
        Some(SAND),
        1,
        [0.8, 0.75, 0.5],
    ),
    block_def(
        WOOD,
        "Wood",
        "Madera",
        "wood",
        true,
        2.0,
        Some(WOOD),
        2,
        [0.4, 0.25, 0.1],
    ),
    block_def(
        LEAVES,
        "Leaves",
        "Hojas",
        "leaves",
        true,
        0.5,
        Some(LEAVES),
        1,
        [0.2, 0.6, 0.2],
    ),
    block_def(
        APPLE,
        "Apple",
        "Manzana",
        "apple",
        true,
        0.5,
        Some(APPLE),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        WHEAT,
        "Wheat",
        "Trigo",
        "wheat",
        true,
        0.5,
        Some(WHEAT),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        MUSHROOM,
        "Mushroom",
        "Seta",
        "mushroom",
        true,
        0.5,
        Some(MUSHROOM),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        WOODEN_DOOR,
        "Wooden Door",
        "Puerta",
        "wooden_door",
        true,
        2.0,
        Some(WOODEN_DOOR),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        STAIRS,
        "Stairs",
        "Escaleras",
        "stairs",
        false,
        2.0,
        Some(STAIRS),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        SLAB,
        "Slab",
        "Losa",
        "slab",
        false,
        2.0,
        Some(SLAB),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        BRICKS,
        "Bricks",
        "Ladrillos",
        "bricks",
        true,
        3.0,
        Some(BRICKS),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        PLANKS,
        "Planks",
        "Tablones",
        "planks",
        true,
        2.0,
        Some(PLANKS),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        TORCH,
        "Torch",
        "Antorcha",
        "torch",
        true,
        0.5,
        Some(TORCH),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        WATER,
        "Water",
        "Agua",
        "water",
        false,
        1.0,
        None,
        0,
        [0.2, 0.5, 0.8],
    ),
    block_def(
        LAVA,
        "Lava",
        "Lava",
        "lava",
        true,
        1.0,
        None,
        0,
        [1.0, 0.5, 0.0],
    ),
    block_def(
        ICE,
        "Ice",
        "Hielo",
        "water",
        true,
        1.0,
        Some(ICE),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        SNOW,
        "Snow",
        "Nieve",
        "grass_top",
        true,
        1.0,
        Some(SNOW),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        OBSIDIAN,
        "Obsidian",
        "Obsidiana",
        "stone",
        true,
        3.0,
        Some(OBSIDIAN),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        MUD,
        "Mud",
        "Barro",
        "dirt",
        true,
        0.8,
        Some(MUD),
        0,
        [0.7, 0.7, 0.7],
    ),
    block_def(
        GRAVEL,
        "Gravel",
        "Grava",
        "sand",
        true,
        0.8,
        Some(GRAVEL),
        0,
        [0.7, 0.7, 0.7],
    ),
    BlockDefinition {
        texture_top: "flower_red",
        texture_side: "flower_red",
        texture_bottom: "flower_red",
        ..block_def(
            RED_FLOWER,
            "Red Flower",
            "Flor roja",
            "flower_red",
            false,
            0.5,
            Some(RED_FLOWER),
            0,
            [0.7, 0.7, 0.7],
        )
    },
    BlockDefinition {
        texture_top: "flower_yellow",
        texture_side: "flower_yellow",
        texture_bottom: "flower_yellow",
        ..block_def(
            YELLOW_FLOWER,
            "Yellow Flower",
            "Flor amarilla",
            "flower_yellow",
            false,
            0.5,
            Some(YELLOW_FLOWER),
            0,
            [0.7, 0.7, 0.7],
        )
    },
    BlockDefinition {
        texture_top: "tall_grass",
        texture_side: "tall_grass",
        texture_bottom: "tall_grass",
        ..block_def(
            TALL_GRASS,
            "Tall Grass",
            "Hierba alta",
            "tall_grass",
            false,
            0.5,
            Some(TALL_GRASS),
            0,
            [0.7, 0.7, 0.7],
        )
    },
    BlockDefinition {
        drop: Some(BlockDropDefinition::Mineral(MineralKind::Coal)),
        requires_pickaxe: true,
        ..block_def(
            COAL_ORE,
            "Coal Ore",
            "Mineral de carbón",
            "coal_ore",
            true,
            2.0,
            None,
            1,
            [0.14, 0.13, 0.12],
        )
    },
    BlockDefinition {
        drop: Some(BlockDropDefinition::Mineral(MineralKind::RawCopper)),
        requires_pickaxe: true,
        ..block_def(
            COPPER_ORE,
            "Copper Ore",
            "Mineral de cobre",
            "copper_ore",
            true,
            3.0,
            None,
            1,
            [0.75, 0.39, 0.22],
        )
    },
    BlockDefinition {
        drop: Some(BlockDropDefinition::Mineral(MineralKind::RawIron)),
        requires_pickaxe: true,
        ..block_def(
            IRON_ORE,
            "Iron Ore",
            "Mineral de hierro",
            "iron_ore",
            true,
            3.0,
            None,
            1,
            [0.65, 0.48, 0.37],
        )
    },
];

pub fn block_definition(id: u8) -> &'static BlockDefinition {
    BLOCKS.get(id as usize).unwrap_or(&BLOCKS[AIR as usize])
}

pub fn block_texture(id: u8, face: usize) -> usize {
    let definition = block_definition(id);
    let texture = match face {
        2 => definition.texture_top,
        3 => definition.texture_bottom,
        _ => definition.texture_side,
    };
    crate::texture_layout::texture_index(texture)
}

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
    block_definition(block).name_en
}

pub fn is_block_solid(
    block_type: u8,
    door_states: &std::collections::HashMap<IVec3, DoorState>,
    pos: IVec3,
) -> bool {
    match block_type {
        WOODEN_DOOR => {
            if let Some(&state) = door_states.get(&pos) {
                !matches!(state, DoorState::Open)
            } else {
                true
            }
        }
        _ => block_definition(block_type).solid,
    }
}
