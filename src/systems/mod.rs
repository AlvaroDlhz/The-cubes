// Game systems
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SurvivalStats {
    pub health: f32,
    pub hunger: f32,
    pub experience: f32,
}

impl Default for SurvivalStats {
    fn default() -> Self {
        Self {
            health: 20.0,
            hunger: 20.0,
            experience: 0.0,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub crouch: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.3,
            on_ground: false,
            crouch: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct PointerState {
    pub locked: bool,
}

#[derive(Resource)]
pub struct SelectedBlock(pub u8);

impl Default for SelectedBlock {
    fn default() -> Self {
        Self(crate::blocks::WOOD)
    }
}

#[derive(Resource, Default)]
pub struct SelectedHotbarSlot(pub usize);

#[derive(Component)]
pub struct HudStatus;

#[derive(Component)]
pub struct ExperienceText;

#[derive(Component)]
pub struct CraftingPanel;

#[derive(Component)]
pub struct InventoryPanel;

#[derive(Component)]
pub struct InventoryCraftingGrid;

#[derive(Component)]
pub struct InventoryGrid;

#[derive(Component)]
pub struct CraftingGridSlot;

#[derive(Component)]
pub struct InventoryCraftResult;

#[derive(Component)]
pub struct HotbarSlotVisual;

#[derive(Component)]
pub struct SurvivalHeartsDisplay;

#[derive(Component)]
pub struct SurvivalHungerDisplay;

#[derive(Component)]
pub struct RecipeBookUI;

#[derive(Resource, Default)]
pub struct CraftingUi {
    pub open: bool,
    pub inventory_open: bool,
    pub just_opened_inventory: bool,
}

#[derive(Clone, Default)]
pub struct Inventory {
    pub blocks: std::collections::HashMap<u8, usize>,
}

impl Inventory {
    pub fn add(&mut self, block: u8) -> bool {
        if !matches!(
            block,
            crate::blocks::GRASS
                | crate::blocks::DIRT
                | crate::blocks::STONE
                | crate::blocks::SAND
                | crate::blocks::WOOD
                | crate::blocks::LEAVES
                | crate::blocks::APPLE
                | crate::blocks::WHEAT
                | crate::blocks::MUSHROOM
                | crate::blocks::WOODEN_DOOR
                | crate::blocks::STAIRS
                | crate::blocks::SLAB
                | crate::blocks::BRICKS
                | crate::blocks::PLANKS
                | crate::blocks::TORCH
        ) {
            return false;
        }
        *self.blocks.entry(block).or_insert(0) += 1;
        true
    }

    pub fn remove(&mut self, block: u8) -> bool {
        if let Some(count) = self.blocks.get_mut(&block) {
            if *count > 1 {
                *count -= 1;
                true
            } else {
                self.blocks.remove(&block);
                true
            }
        } else {
            false
        }
    }

    pub fn count(&self, block: u8) -> usize {
        self.blocks.get(&block).copied().unwrap_or(0)
    }
}
