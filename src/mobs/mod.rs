// Mob systems and components
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MobKind {
    Grazer,
    NightStalker,
}

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Mob {
    pub kind: MobKind,
    pub health: f32,
    pub attack_elapsed: f32,
    pub wander_elapsed: f32,
    pub wander_angle: f32,
    pub wander_step: u32,
}

impl Default for Mob {
    fn default() -> Self {
        Self {
            kind: MobKind::Grazer,
            health: 10.0,
            attack_elapsed: 0.0,
            wander_elapsed: 0.0,
            wander_angle: 0.0,
            wander_step: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct MobPopulation {
    pub total_count: usize,
    pub grazer_count: usize,
    pub stalker_count: usize,
}

#[derive(Resource, Default)]
pub struct MobAssets {
    pub grazer_mesh: Handle<Mesh>,
    pub stalker_mesh: Handle<Mesh>,
    pub grazer_material: Handle<StandardMaterial>,
    pub stalker_material: Handle<StandardMaterial>,
}

pub const MOB_DESPAWN_DISTANCE: f32 = 96.0;
pub const MOB_SPAWN_DISTANCE: f32 = 64.0;
pub const MOB_SPAWN_CHANCE: f32 = 0.2;
pub const MOB_SPAWN_CAP_GRAZER: usize = 16;
pub const MOB_SPAWN_CAP_STALKER: usize = 4;
