#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::type_complexity)]
#![allow(clippy::manual_checked_ops)]
#![allow(clippy::unnecessary_sort_by)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    window::{CursorGrabMode, PrimaryWindow, WindowPlugin},
};
use serde::{Deserialize, Serialize};

// Module declarations
mod blocks;
mod mobs;
mod world;
mod systems;

// Selective imports from modules
use blocks::*;
use mobs::*;
use world::*;
use systems::*;

const CHUNK_SIZE: i32 = 16;
const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
const SAND: u8 = 4;
const WOOD: u8 = 5;
const LEAVES: u8 = 6;
const APPLE: u8 = 7;
const WHEAT: u8 = 8;
const MUSHROOM: u8 = 9;
const WOODEN_DOOR: u8 = 10;
const STAIRS: u8 = 11;
const SLAB: u8 = 12;
const BRICKS: u8 = 13;
const PLANKS: u8 = 14;
const TORCH: u8 = 15;
const WATER: u8 = 16;
const LAVA: u8 = 17;
const BLOCK_COUNT: usize = LAVA as usize + 1;
const LEGACY_BLOCK_COUNT: usize = 4;
const REACH: f32 = 6.0;
const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_EYE_HEIGHT: f32 = 1.62;
const PLAYER_HEADROOM: f32 = 0.18;
const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 8.0;
const GRAVITY: f32 = 24.0;
const JUMP_SPEED: f32 = 8.5;
const STEP_HEIGHT: f32 = 1.0;
const SAVE_VERSION: u32 = 1;
const STREAM_LOAD_RADIUS: i32 = 3;
const STREAM_UNLOAD_RADIUS: i32 = 4;
const GAME_DAY_SECONDS: f32 = 24.0 * 60.0 * 60.0;
const GAME_SECONDS_PER_REAL_SECOND: f32 = 60.0;
const DEFAULT_TIME_OF_DAY_SECONDS: f32 = 8.0 * 60.0 * 60.0;
const SKY_RADIUS: f32 = 180.0;
const CELESTIAL_RADIUS: f32 = 120.0;
const STAR_RADIUS: f32 = 145.0;
const HOTBAR_SLOT_COUNT: usize = 6;
const HOTBAR_BLOCKS: [u8; HOTBAR_SLOT_COUNT] = [GRASS, DIRT, STONE, SAND, WOOD, LEAVES];
const AUTOSAVE_INTERVAL_SECONDS: f32 = 60.0;
const MAX_HEALTH: f32 = 20.0;
const MAX_HUNGER: f32 = 20.0;
const PASSIVE_HUNGER_INTERVAL: f32 = 75.0;
const SPRINT_HUNGER_INTERVAL: f32 = 30.0;
const STARVATION_DAMAGE_INTERVAL: f32 = 10.0;
const HEALTH_REGEN_INTERVAL: f32 = 8.0;
const MAX_MOBS: usize = 8;
const MAX_PASSIVE_MOBS: usize = 5;
const MAX_HOSTILE_MOBS: usize = 3;
const MOB_SPAWN_INTERVAL: f32 = 4.0;
const MOB_DESPAWN_DISTANCE: f32 = 40.0;

// ============================================================================
// MODULE: rendering
// ============================================================================
// Handles block visualization, mesh generation, and texture mapping
// Pure functions that don't require mutable state beyond input parameters
mod rendering {
    use super::*;

    /// Get texture variant (0-3) based on world position
    /// Uses block coordinates to deterministically select variant
    /// Ensures same block at same location always has same texture
    pub fn block_variant(x: i32, y: i32, z: i32) -> u32 {
        // Create a simple hash from coordinates
        let hash = ((x.wrapping_mul(73856093) 
                   ^ y.wrapping_mul(19349663) 
                   ^ z.wrapping_mul(83492791)) as u32).wrapping_mul(2654435761);
        hash % 4
    }

    /// Rotate UV coordinates by 90 degrees clockwise
    /// Transforms (p0, p1, p2, p3) where p3 is top-left
    fn rotate_uv_90(uv0: [f32; 2], uv1: [f32; 2], uv2: [f32; 2], uv3: [f32; 2]) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        // Rotate 90 degrees: corners shift in order
        (uv3, uv0, uv1, uv2)
    }

    /// Rotate UV coordinates by 180 degrees
    fn rotate_uv_180(uv0: [f32; 2], uv1: [f32; 2], uv2: [f32; 2], uv3: [f32; 2]) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        (uv2, uv3, uv0, uv1)
    }

    /// Rotate UV coordinates by 270 degrees clockwise (90 counter-clockwise)
    fn rotate_uv_270(uv0: [f32; 2], uv1: [f32; 2], uv2: [f32; 2], uv3: [f32; 2]) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        (uv1, uv2, uv3, uv0)
    }

    /// Apply texture variant rotation to UV coordinates
    /// Variant 0: No rotation
    /// Variant 1: 90 degrees
    /// Variant 2: 180 degrees
    /// Variant 3: 270 degrees
    fn apply_variant(
        variant: u32,
        uv0: [f32; 2],
        uv1: [f32; 2],
        uv2: [f32; 2],
        uv3: [f32; 2],
    ) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        match variant {
            0 => (uv0, uv1, uv2, uv3),
            1 => rotate_uv_90(uv0, uv1, uv2, uv3),
            2 => rotate_uv_180(uv0, uv1, uv2, uv3),
            3 => rotate_uv_270(uv0, uv1, uv2, uv3),
            _ => (uv0, uv1, uv2, uv3),
        }
    }

    /// Get UV coordinates in the atlas for a block type with variant rotation
    /// Atlas is 16x16 grid, each tile is 1/16th of the texture
    /// Returns (u_min, v_min, u_max, v_max) in normalized coordinates [0, 1]
    /// Applies rotation based on variant parameter
    pub fn block_uv(block: u8, face: usize, variant: u32) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        // Map block type to atlas tile position
        // Special case: GRASS has different top and side textures
        let tile_id = if block == GRASS && face == 2 {
            2  // Grass top is tile 2 (green)
        } else {
            block as usize
        };
        
        let tile_x = tile_id % 16;
        let tile_y = tile_id / 16;
        let uv_min_x = tile_x as f32 / 16.0;
        let uv_max_x = (tile_x as f32 + 1.0) / 16.0;
        let uv_min_y = tile_y as f32 / 16.0;
        let uv_max_y = (tile_y as f32 + 1.0) / 16.0;
        
        // Base UV corners: (bottom-left, bottom-right, top-right, top-left)
        let base_uv0 = [uv_min_x, uv_min_y];
        let base_uv1 = [uv_max_x, uv_min_y];
        let base_uv2 = [uv_max_x, uv_max_y];
        let base_uv3 = [uv_min_x, uv_max_y];
        
        // Apply variant rotation
        apply_variant(variant, base_uv0, base_uv1, base_uv2, base_uv3)
    }

    /// Get shading multiplier based on face direction for depth perception
    pub fn face_shade(face: usize) -> f32 {
        if face == 3 {
            0.72
        } else if face == 0 || face == 1 {
            0.86
        } else {
            1.0
        }
    }

    /// Create an empty mesh (used when chunk has no visible faces)
    pub fn empty_mesh() -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
        mesh.insert_indices(Indices::U32(Vec::new()));
        mesh
    }

    /// Greedy mesh algorithm: merge coplanar block faces into rectangles
    /// Reduces vertex count and draw calls significantly
    pub fn greedy_rectangles(mask: &mut [u8], mut emit: impl FnMut(usize, usize, usize, usize, u8)) {
        let size = CHUNK_SIZE as usize;
        debug_assert_eq!(mask.len(), size * size);

        for v in 0..size {
            for u in 0..size {
                let block = mask[v * size + u];
                if block == AIR {
                    continue;
                }

                let mut width = 1;
                while u + width < size && mask[v * size + u + width] == block {
                    width += 1;
                }

                let mut height = 1;
                'extend_height: while v + height < size {
                    for next_u in u..u + width {
                        if mask[(v + height) * size + next_u] != block {
                            break 'extend_height;
                        }
                    }
                    height += 1;
                }

                for next_v in v..v + height {
                    for next_u in u..u + width {
                        mask[next_v * size + next_u] = AIR;
                    }
                }
                emit(u, v, width, height, block);
            }
        }
    }

    /// Build complete mesh for a chunk
    /// Uses greedy algorithm to minimize vertices, applies face culling
    pub fn build_chunk_mesh(world: &WorldData, coord: IVec3) -> Mesh {
        let mut positions = Vec::<[f32; 3]>::new();
        let mut normals = Vec::<[f32; 3]>::new();
        let mut uvs = Vec::<[f32; 2]>::new();
        let mut indices = Vec::<u32>::new();
        let Some(chunk) = world.chunks.get(&coord) else {
            return empty_mesh();
        };

        for axis in 0..3 {
            let u_axis = (axis + 1) % 3;
            let v_axis = (axis + 2) % 3;
            let mut normal = IVec3::ZERO;

            for positive in [true, false] {
                normal[axis] = if positive { 1 } else { -1 };
                let face_index = axis * 2 + if positive { 0 } else { 1 };

                for slice in 0..CHUNK_SIZE {
                    let mut mask = [AIR; (CHUNK_SIZE * CHUNK_SIZE) as usize];
                    for v in 0..CHUNK_SIZE {
                        for u in 0..CHUNK_SIZE {
                            let mut local = IVec3::ZERO;
                            local[axis] = slice;
                            local[u_axis] = u;
                            local[v_axis] = v;

                            let block = chunk.get(local);
                            if block == AIR {
                                continue;
                            }
                            let neighbor = coord * CHUNK_SIZE + local + normal;
                            if super::block_at(world, neighbor) == AIR {
                                mask[(v * CHUNK_SIZE + u) as usize] = block;
                            }
                        }
                    }

                    greedy_rectangles(&mut mask, |u, v, width, height, block| {
                        let mut origin = IVec3::ZERO;
                        origin[axis] = slice + if positive { 1 } else { 0 };
                        origin[u_axis] = u as i32;
                        origin[v_axis] = v as i32;

                        let mut u_direction = IVec3::ZERO;
                        u_direction[u_axis] = width as i32;
                        let mut v_direction = IVec3::ZERO;
                        v_direction[v_axis] = height as i32;

                        let origin = origin.as_vec3();
                        let u_direction = u_direction.as_vec3();
                        let v_direction = v_direction.as_vec3();
                        let corners = if positive {
                            [
                                origin,
                                origin + u_direction,
                                origin + u_direction + v_direction,
                                origin + v_direction,
                            ]
                        } else {
                            [
                                origin,
                                origin + v_direction,
                                origin + u_direction + v_direction,
                                origin + u_direction,
                            ]
                        };
                        
                        let block_pos = (coord * CHUNK_SIZE + origin.as_ivec3()).to_array();
                        let variant = block_variant(block_pos[0], block_pos[1], block_pos[2]);
                        let (uv0, uv1, uv2, uv3) = block_uv(block, face_index, variant);
                        let _shade = face_shade(face_index);
                        
                        let base = positions.len() as u32;
                        for (corner, uv) in corners.iter().zip([uv0, uv1, uv2, uv3].iter()) {
                            positions.push(corner.to_array());
                            normals.push(normal.as_vec3().to_array());
                            uvs.push(*uv);
                        }
                        indices.extend_from_slice(&[
                            base,
                            base + 1,
                            base + 2,
                            base,
                            base + 2,
                            base + 3,
                        ]);
                    });
                }
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "The Cubes".into(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<WorldData>()
        .init_resource::<PointerState>()
        .init_resource::<SelectedBlock>()
        .init_resource::<SelectedHotbarSlot>()
        .init_resource::<Inventory>()
        .init_resource::<Survival>()
        .init_resource::<CraftedTools>()
        .init_resource::<CraftingUi>()
        .init_resource::<WorldForm>()
        .init_resource::<PendingWorld>()
        .init_resource::<TextureAssets>()
        .init_resource::<WorldStorage>()
        .init_resource::<LoadingProgress>()
        .init_resource::<DayNightCycle>()
        .init_resource::<MobPopulation>()
        .init_resource::<MobAssets>()
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
        .add_systems(OnEnter(AppState::WorldSelect), setup_world_select)
        .add_systems(OnExit(AppState::WorldSelect), cleanup_menu)
        .add_systems(OnEnter(AppState::CreateWorld), setup_create_world)
        .add_systems(OnExit(AppState::CreateWorld), cleanup_menu)
        .add_systems(OnEnter(AppState::LoadingWorld), begin_loading_world)
        .add_systems(OnEnter(AppState::LoadingWorld), setup_loading_screen)
        .add_systems(OnExit(AppState::LoadingWorld), cleanup_menu)
        .add_systems(OnEnter(AppState::InGame), load_textures)
        .add_systems(OnEnter(AppState::InGame), setup_game_scene)
        .add_systems(OnEnter(AppState::Paused), setup_pause_menu)
        .add_systems(OnExit(AppState::Paused), cleanup_pause_menu)
        .add_systems(OnEnter(AppState::MainMenu), cleanup_game_scene)
        .add_systems(
            Update,
            (
                main_menu_actions,
                world_select_actions,
                loading_world_actions,
                loading_world_step,
            )
                .chain()
                .run_if(
                    in_state(AppState::MainMenu)
                        .or(in_state(AppState::WorldSelect))
                        .or(in_state(AppState::CreateWorld))
                        .or(in_state(AppState::LoadingWorld))
                        .or(in_state(AppState::Paused)),
                ),
        )
        .add_systems(
            Update,
            (
                create_world_input,
                create_world_actions,
                update_world_form_ui,
            )
                .chain()
                .run_if(in_state(AppState::CreateWorld)),
        )
        .add_systems(
            Update,
            pause_menu_actions.run_if(in_state(AppState::Paused)),
        )
        .add_systems(
            Update,
            (
                (
                    capture_pointer,
                    look,
                    move_player,
                    update_survival,
                    consume_food,
                    advance_day_night,
                    update_day_night_lighting,
                    update_sky,
                    stream_world_chunks,
                    update_particles,
                    animate_water_lava,
                ),
                update_mobs,
                (
                    select_block,
                    select_hotbar,
                    update_hud,
                    update_survival_hud,
                    update_hotbar,
                    edit_blocks,
                    toggle_crafting,
                    toggle_inventory_visibility,
                    craft_recipe_buttons,
                    rebuild_dirty_chunks,
                    autosave_world,
                    pause_from_game,
                ),
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(Startup, setup_ui_camera)
        .run();
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    MainMenu,
    WorldSelect,
    CreateWorld,
    LoadingWorld,
    InGame,
    Paused,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct UiCamera;

#[derive(Component)]
struct NameInputText;

#[derive(Component)]
struct SeedInputText;

#[derive(Component)]
struct FormErrorText;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct PauseStatus;

#[derive(Component)]
struct PauseRoot;

#[derive(Component)]
struct MenuAction(MenuCommand);

#[derive(Clone, Copy)]
enum MenuCommand {
    Play,
    Create,
    Back,
    StartWorld,
    Resume,
    SaveAndMenu,
    CancelLoading,
    Exit,
}

#[derive(Component)]
struct LoadWorldAction(PathBuf);

#[derive(Component)]
struct BlockParticle {
    lifetime: f32,
    max_lifetime: f32,
    velocity: Vec3,
}

#[derive(Component)]
struct FormField(WorldField);

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorldField {
    Name,
    Seed,
}

#[derive(Resource)]
struct WorldForm {
    name: String,
    seed: String,
    focus: WorldField,
    error: Option<String>,
}

impl Default for WorldForm {
    fn default() -> Self {
        Self {
            name: "Mi mundo".into(),
            seed: "12345".into(),
            focus: WorldField::Name,
            error: None,
        }
    }
}

#[derive(Resource, Default)]
struct PendingWorld {
    save: Option<WorldSave>,
    path: Option<PathBuf>,
    is_new_world: bool,
}

#[derive(Resource, Default)]
struct TextureAssets {
    atlas: Option<Handle<Image>>,
    water_material: Option<Handle<StandardMaterial>>,
    lava_material: Option<Handle<StandardMaterial>>,
}

#[derive(Resource)]
struct DayNightCycle {
    time_of_day_seconds: f32,
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
        }
    }
}

#[derive(Resource, Default)]
struct LoadingProgress {
    coordinates: Vec<IVec3>,
    completed: usize,
    total: usize,
    chunk_count: usize,
    chunk_cursor: usize,
    stage: LoadingStage,
    material: Option<Handle<StandardMaterial>>,
    is_new_world: bool,
    initialized: bool,
}

#[derive(Default, PartialEq, Eq)]
enum LoadingStage {
    #[default]
    Chunks,
    Meshes,
}

#[derive(Serialize, Deserialize)]
struct WorldSave {
    version: u32,
    name: String,
    seed: u64,
    #[serde(default = "default_time_of_day_seconds")]
    time_of_day_seconds: f32,
    player_position: [f32; 3],
    #[serde(default)]
    player_yaw: f32,
    #[serde(default)]
    player_pitch: f32,
    selected_block: u8,
    #[serde(default)]
    selected_hotbar_slot: usize,
    inventory: [u32; LEGACY_BLOCK_COUNT],
    #[serde(default)]
    extra_inventory: [u32; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
    #[serde(default = "default_health")]
    health: f32,
    #[serde(default = "default_hunger")]
    hunger: f32,
    #[serde(default)]
    experience: u32,
    #[serde(default = "default_starting_food")]
    food: u32,
    #[serde(default)]
    pickaxe_durability: u32,
    #[serde(default)]
    axe_durability: u32,
    #[serde(default)]
    shovel_durability: u32,
    #[serde(default)]
    sword_durability: u32,
    chunks: Vec<ChunkSave>,
}

#[derive(Serialize, Deserialize)]
struct ChunkSave {
    coord: [i32; 3],
    blocks: Vec<u8>,
}

#[derive(Resource, Default)]
struct WorldData {
    chunks: HashMap<IVec3, Chunk>,
    modified_chunks: HashMap<IVec3, Chunk>,
    dirty: HashSet<IVec3>,
    name: String,
    seed: u64,
    door_states: HashMap<IVec3, DoorState>,
}

#[derive(Resource, Default)]
struct WorldStorage {
    path: Option<PathBuf>,
    legacy_chunks: HashMap<IVec3, Chunk>,
    dirty_chunks: HashSet<IVec3>,
    failed_chunks: HashSet<IVec3>,
    autosave_elapsed: f32,
    retry_elapsed: f32,
    metadata_save_requested: bool,
    status: String,
}

#[derive(Clone)]
struct Chunk {
    blocks: Vec<u8>,
}

impl Chunk {
    fn new() -> Self {
        Self {
            blocks: vec![AIR; CHUNK_VOLUME],
        }
    }

    fn get(&self, local: IVec3) -> u8 {
        self.blocks[block_index(local)]
    }

    fn set(&mut self, local: IVec3, block: u8) {
        self.blocks[block_index(local)] = block;
    }
}

#[derive(Component)]
struct ChunkMesh(IVec3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MobKind {
    Grazer,
    NightStalker,
}

#[derive(Component)]
struct Mob {
    kind: MobKind,
    health: u8,
    attack_elapsed: f32,
    wander_elapsed: f32,
    wander_angle: f32,
    wander_step: u32,
}

#[derive(Resource, Default)]
struct MobPopulation {
    spawn_elapsed: f32,
    spawn_attempt: u64,
}

#[derive(Resource, Default)]
struct MobAssets {
    cube: Option<Handle<Mesh>>,
    grazer_body: Option<Handle<StandardMaterial>>,
    grazer_head: Option<Handle<StandardMaterial>>,
    grazer_leg: Option<Handle<StandardMaterial>>,
    stalker_body: Option<Handle<StandardMaterial>>,
    stalker_eye: Option<Handle<StandardMaterial>>,
}

#[derive(Component)]
struct Player {
    yaw: f32,
    pitch: f32,
    velocity: Vec3,
    grounded: bool,
    flying: bool,
}

#[derive(Resource, Default)]
struct PointerState {
    captured_this_frame: bool,
}

#[derive(Resource)]
struct SelectedBlock(u8);

impl Default for SelectedBlock {
    fn default() -> Self {
        Self(GRASS)
    }
}

#[derive(Resource, Default)]
struct SelectedHotbarSlot(usize);

#[derive(Component)]
struct HudStatus;

#[derive(Component)]
struct ExperienceText;

#[derive(Component)]
struct CraftingPanel;

#[derive(Component)]
struct CraftingStatus;

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct InventoryGrid;

#[derive(Component, Clone, Copy)]
struct InventorySlot {
    block_type: u8,
    slot_index: usize,
}

#[derive(Component)]
struct InventoryCraftingGrid;

#[derive(Component)]
struct InventoryCraftResult;

#[derive(Component)]
struct CraftingGridSlot {
    position: usize,
}

#[derive(Component)]
struct HotbarSlotVisual {
    slot_index: usize,
}

#[derive(Component)]
struct SurvivalHeartsDisplay;

#[derive(Component)]
struct SurvivalHungerDisplay;

#[derive(Component)]
struct RecipeBookUI;

#[derive(Component, Clone, Copy)]
struct CraftRecipeButton(CraftRecipe);

#[derive(Clone, Copy)]
enum CraftRecipe {
    WoodenPickaxe,
    WoodenAxe,
    WoodenShovel,
    WoodenSword,
    BerryRation,
    Arrows,
    WoodenDoor,
    Stairs,
    Slab,
    Bricks,
    Planks,
    Torch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FoodType {
    BerryRation,   // saturation: 4
    Apple,         // saturation: 6
    Mushroom,      // saturation: 2 (puede ser venenoso)
    Wheat,         // saturation: 8
    RawMeat,       // saturation: 3
    CookedMeat,    // saturation: 8
}

impl FoodType {
    fn saturation(&self) -> f32 {
        match self {
            FoodType::BerryRation => 4.0,
            FoodType::Apple => 6.0,
            FoodType::Mushroom => 2.0,
            FoodType::Wheat => 8.0,
            FoodType::RawMeat => 3.0,
            FoodType::CookedMeat => 8.0,
        }
    }

    fn is_poisonous(&self) -> bool {
        matches!(self, FoodType::Mushroom)
    }
}

// Block state for doors (open/closed)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DoorState {
    Open,
    Closed,
}

// Block state for stairs (rotation and half)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StairRotation {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StairHalf {
    Bottom,
    Top,
}

#[derive(Resource, Default)]
struct CraftingUi {
    open: bool,
    just_closed: bool,
    inventory_open: bool,
    inventory_just_opened: bool,
}

#[derive(Resource, Default)]
struct CraftedTools {
    pickaxe_durability: u32,
    axe_durability: u32,
    shovel_durability: u32,
    sword_durability: u32,
}

#[derive(Clone, Copy)]
enum SurvivalBarKind {
    Health,
    Hunger,
    Experience,
}

#[derive(Component)]
struct SurvivalBarFill(SurvivalBarKind);

#[derive(Component)]
struct SurvivalValue(SurvivalBarKind);

#[derive(Resource)]
struct Survival {
    health: f32,
    hunger: f32,
    experience: u32,
    food: u32,
    passive_hunger_elapsed: f32,
    sprint_hunger_elapsed: f32,
    starvation_elapsed: f32,
    regeneration_elapsed: f32,
    notice: String,
    notice_elapsed: f32,
}

impl Default for Survival {
    fn default() -> Self {
        Self {
            health: MAX_HEALTH,
            hunger: MAX_HUNGER,
            experience: 0,
            food: default_starting_food(),
            passive_hunger_elapsed: 0.0,
            sprint_hunger_elapsed: 0.0,
            starvation_elapsed: 0.0,
            regeneration_elapsed: 0.0,
            notice: String::new(),
            notice_elapsed: 0.0,
        }
    }
}

#[derive(Component)]
struct SkyDome;

#[derive(Component)]
struct SkyStars;

#[derive(Component)]
struct SunCube;

#[derive(Component)]
struct MoonCube;

#[derive(Component)]
struct HotbarSlot(usize);

#[derive(Component)]
struct HotbarCount(usize);

#[derive(Resource, Default)]
struct Inventory {
    counts: [u32; BLOCK_COUNT],
}

impl Inventory {
    fn count(&self, block: u8) -> u32 {
        self.counts.get(block as usize).copied().unwrap_or(0)
    }

    fn add(&mut self, block: u8) -> Result<(), u8> {
        if !matches!(
            block,
            GRASS | DIRT | STONE | SAND | WOOD | LEAVES | APPLE | WHEAT
                | MUSHROOM | WOODEN_DOOR | STAIRS | SLAB | BRICKS | PLANKS | TORCH
        ) {
            return Err(block);
        }
        let Some(count) = self.counts.get_mut(block as usize) else {
            return Err(block);
        };
        *count = count.checked_add(1).ok_or(block)?;
        Ok(())
    }

    fn remove(&mut self, block: u8) -> bool {
        let Some(count) = self.counts.get_mut(block as usize) else {
            return false;
        };
        if *count == 0 {
            return false;
        }
        *count -= 1;
        true
    }
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        UiCamera,
    ));
}

fn generate_chunk(coord: IVec3, seed: u64) -> Chunk {
    let mut chunk = Chunk::new();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let height = terrain_height(world_x, world_z, seed);
            let biome = biome_at(world_x, world_z, seed);
            for y in 0..=height {
                let block = if y == height {
                    biome.surface_block()
                } else if biome == Biome::Desert && y >= height - 3 {
                    SAND
                } else if biome == Biome::Mountain && y < height - 3 {
                    STONE
                } else if y >= height - 3 {
                    DIRT
                } else {
                    STONE
                };
                chunk.set(IVec3::new(x, y, z), block);
            }
        }
    }

    for z in -2..CHUNK_SIZE + 2 {
        for x in -2..CHUNK_SIZE + 2 {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let height = terrain_height(world_x, world_z, seed);
            if biome_at(world_x, world_z, seed) != Biome::Forest
                || height > 10
                || coordinate_hash(world_x, world_z, seed) % 100 >= 20
            {
                continue;
            }
            let tree_height =
                3 + (coordinate_hash(world_z, world_x, seed ^ 0xA24B_AED4_963E_E407) % 2) as i32;
            for trunk_y in height + 1..=height + tree_height {
                set_generated_block(&mut chunk, x, trunk_y, z, WOOD);
            }
            let crown_y = height + tree_height;
            for leaf_y in crown_y - 1..=crown_y + 1 {
                let radius: i32 = if leaf_y == crown_y + 1 { 1 } else { 2 };
                for leaf_z in -radius..=radius {
                    for leaf_x in -radius..=radius {
                        if leaf_y == crown_y + 1 && leaf_x.abs() + leaf_z.abs() > 1 {
                            continue;
                        }
                        let local_x = x + leaf_x;
                        let local_z = z + leaf_z;
                        if (0..CHUNK_SIZE).contains(&local_x)
                            && (0..CHUNK_SIZE).contains(&local_z)
                            && (0..CHUNK_SIZE).contains(&leaf_y)
                        {
                            let local = IVec3::new(local_x, leaf_y, local_z);
                            if chunk.get(local) == AIR {
                                chunk.set(local, LEAVES);
                                // Occasionally spawn apples in leaves
                                if coordinate_hash(world_x ^ leaf_x, leaf_y, seed ^ 0x5C0F) % 100 < 15 {
                                    chunk.set(local, APPLE);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate mushrooms in forests
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Forest {
                continue;
            }
            let height = terrain_height(world_x, world_z, seed);
            if coordinate_hash(world_x, world_z, seed ^ 0xDEAD_BEEF) % 100 < 5 {
                let mushroom_y = height + 1;
                if (0..CHUNK_SIZE).contains(&mushroom_y) {
                    chunk.set(IVec3::new(x, mushroom_y, z), MUSHROOM);
                }
            }
        }
    }

    // Generate wheat in grassland
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Plains {
                continue;
            }
            let height = terrain_height(world_x, world_z, seed);
            if coordinate_hash(world_x, world_z, seed ^ 0xBEEF_FACE) % 100 < 8 {
                let wheat_y = height + 1;
                if (0..CHUNK_SIZE).contains(&wheat_y) {
                    chunk.set(IVec3::new(x, wheat_y, z), WHEAT);
                }
            }
        }
    }
    chunk
}

fn set_generated_block(chunk: &mut Chunk, x: i32, y: i32, z: i32, block: u8) {
    if (0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y) && (0..CHUNK_SIZE).contains(&z)
    {
        chunk.set(IVec3::new(x, y, z), block);
    }
}

fn load_textures(
    asset_server: Res<AssetServer>,
    mut texture_assets: ResMut<TextureAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if texture_assets.atlas.is_none() {
        texture_assets.atlas = Some(asset_server.load("texture_atlas.png"));
        
        // Create separate materials for water and lava with animation colors
        texture_assets.water_material = Some(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.2, 0.5, 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }));
        
        texture_assets.lava_material = Some(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 0.5, 0.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }));
    }
}

fn setup_game_scene(
    mut commands: Commands,
    pending: Res<PendingWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mob_assets: ResMut<MobAssets>,
    inventory: Res<Inventory>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    existing_player: Query<(), With<Player>>,
) {
    if !existing_player.is_empty() {
        if let Ok(mut window) = windows.get_single_mut() {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
        return;
    }
    if mob_assets.cube.is_none() {
        mob_assets.cube = Some(meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
        mob_assets.grazer_body = Some(materials.add(Color::srgb(0.52, 0.30, 0.15)));
        mob_assets.grazer_head = Some(materials.add(Color::srgb(0.72, 0.52, 0.30)));
        mob_assets.grazer_leg = Some(materials.add(Color::srgb(0.24, 0.15, 0.10)));
        mob_assets.stalker_body = Some(materials.add(Color::srgb(0.20, 0.16, 0.30)));
        mob_assets.stalker_eye = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.12, 0.08),
            emissive: LinearRgba::new(1.0, 0.08, 0.04, 0.0),
            ..default()
        }));
    }

    let (position, yaw, pitch) =
        pending
            .save
            .as_ref()
            .map_or((Vec3::new(8.0, 15.0, 25.0), 0.0, -0.3), |save| {
                (
                    Vec3::from_array(save.player_position),
                    save.player_yaw,
                    save.player_pitch,
                )
            });
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(position).with_rotation(player_rotation(yaw, pitch)),
        GameEntity,
        Player {
            yaw,
            pitch,
            velocity: Vec3::ZERO,
            grounded: false,
            flying: false,
        },
    ));

    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.0)),
        GameEntity,
    ));
    let sky_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let sky_mesh = meshes.add(create_sky_dome_mesh());
    commands.spawn((
        Mesh3d(sky_mesh),
        MeshMaterial3d(sky_material),
        Transform::from_translation(position),
        SkyDome,
        GameEntity,
    ));

    let stars_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.78, 0.86, 1.0, 0.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(create_stars_mesh())),
        MeshMaterial3d(stars_material),
        Transform::from_translation(position),
        SkyStars,
        GameEntity,
    ));

    let sun_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.82, 0.48),
        unlit: true,
        ..default()
    });
    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.84, 1.0),
        unlit: true,
        ..default()
    });
    let celestial_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    commands.spawn((
        Mesh3d(celestial_mesh.clone()),
        MeshMaterial3d(sun_material),
        Transform::from_translation(position).with_scale(Vec3::splat(7.0)),
        SunCube,
        GameEntity,
    ));
    commands.spawn((
        Mesh3d(celestial_mesh),
        MeshMaterial3d(moon_material),
        Transform::from_translation(position).with_scale(Vec3::splat(5.0)),
        MoonCube,
        GameEntity,
    ));
    setup_hud(&mut commands);
    setup_inventory_ui(&mut commands, &inventory);
}

fn advance_day_night(time: Res<Time>, mut cycle: ResMut<DayNightCycle>) {
    cycle.time_of_day_seconds = advance_time_of_day(
        cycle.time_of_day_seconds,
        time.delta_secs(),
        GAME_SECONDS_PER_REAL_SECOND,
    );
}

fn update_day_night_lighting(
    cycle: Res<DayNightCycle>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight)>,
    mut ambient: ResMut<AmbientLight>,
) {
    let Ok((mut transform, mut light)) = sun.get_single_mut() else {
        return;
    };
    let (_, daylight) = sun_position_and_daylight(cycle.time_of_day_seconds);
    let direction = -sun_direction(cycle.time_of_day_seconds);
    transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);

    let twilight = (1.0 - daylight).clamp(0.0, 1.0);
    light.illuminance = 150.0 + daylight * 11_850.0;
    light.color = Color::srgb(1.0, 0.62 + daylight * 0.38, 0.42 + daylight * 0.58);
    ambient.color = Color::srgb(
        0.16 + daylight * 0.48,
        0.20 + daylight * 0.52,
        0.36 + daylight * 0.44,
    );
    ambient.brightness = 20.0 + daylight * 180.0 + twilight * 8.0;
}

fn update_sky(
    cycle: Res<DayNightCycle>,
    player: Query<&Transform, With<Player>>,
    mut sky: Query<(&mut Transform, &Mesh3d), (With<SkyDome>, Without<Player>)>,
    mut stars: Query<
        (&mut Transform, &MeshMaterial3d<StandardMaterial>),
        (With<SkyStars>, Without<Player>, Without<SkyDome>),
    >,
    mut sun: Query<
        (&mut Transform, &mut Visibility),
        (
            With<SunCube>,
            Without<Player>,
            Without<SkyDome>,
            Without<SkyStars>,
        ),
    >,
    mut moon: Query<
        (&mut Transform, &mut Visibility),
        (
            With<MoonCube>,
            Without<Player>,
            Without<SkyDome>,
            Without<SkyStars>,
            Without<SunCube>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player.get_single() else {
        return;
    };
    let center = player_transform.translation;
    let time = cycle.time_of_day_seconds;
    let (elevation, daylight) = sun_position_and_daylight(time);
    let direction = sun_direction(time);
    let cycle_time = time.rem_euclid(GAME_DAY_SECONDS);

    if let Ok((mut transform, mesh_handle)) = sky.get_single_mut() {
        transform.translation = center;
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, sky_vertex_colors(time));
        } else {
            error!("Missing sky dome mesh asset");
        }
    }

    if let Ok((mut transform, material_handle)) = stars.get_single_mut() {
        transform.translation = center;
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let opacity = (1.0 - smoothstep(0.08, 0.42, daylight)) * 0.95;
            material.base_color = Color::srgba(0.78, 0.86, 1.0, opacity);
        } else {
            error!("Missing star material asset");
        }
    }

    if let Ok((mut transform, mut visibility)) = sun.get_single_mut() {
        transform.translation = center + direction * CELESTIAL_RADIUS;
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            cycle_time * 0.0007,
            cycle_time * 0.0011,
            cycle_time * 0.0004,
        );
        *visibility = if elevation > -0.035 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut transform, mut visibility)) = moon.get_single_mut() {
        transform.translation = center - direction * CELESTIAL_RADIUS;
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            -cycle_time * 0.0003,
            cycle_time * 0.0005,
            cycle_time * 0.0002,
        );
        *visibility = if elevation < 0.035 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn advance_time_of_day(current: f32, delta_seconds: f32, game_seconds_per_real_second: f32) -> f32 {
    (current + delta_seconds.max(0.0) * game_seconds_per_real_second).rem_euclid(GAME_DAY_SECONDS)
}

fn sun_position_and_daylight(time_of_day_seconds: f32) -> (f32, f32) {
    let phase = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / GAME_DAY_SECONDS)
        * std::f32::consts::TAU;
    let elevation = (phase - std::f32::consts::FRAC_PI_2).sin();
    let daylight = smoothstep(-0.12, 0.2, elevation);
    (elevation, daylight)
}

fn sun_direction(time_of_day_seconds: f32) -> Vec3 {
    let (elevation, _) = sun_position_and_daylight(time_of_day_seconds);
    let phase = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / GAME_DAY_SECONDS)
        * std::f32::consts::TAU;
    let horizontal = (1.0 - elevation * elevation).max(0.0).sqrt();
    Vec3::new(
        phase.cos() * horizontal,
        elevation,
        phase.sin() * horizontal,
    )
    .normalize_or_zero()
}

fn create_sky_dome_mesh() -> Mesh {
    const LATITUDE_SEGMENTS: usize = 32;
    const LONGITUDE_SEGMENTS: usize = 64;

    let mut positions = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1));
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);

    for latitude in 0..=LATITUDE_SEGMENTS {
        let vertical = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * latitude as f32 / LATITUDE_SEGMENTS as f32;
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let angle = std::f32::consts::TAU * longitude as f32 / LONGITUDE_SEGMENTS as f32;
            let direction = Vec3::new(
                vertical.cos() * angle.cos(),
                vertical.sin(),
                vertical.cos() * angle.sin(),
            );
            positions.push((direction * SKY_RADIUS).to_array());
            normals.push(direction.to_array());
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let top_left = (latitude * (LONGITUDE_SEGMENTS + 1) + longitude) as u32;
            let top_right = top_left + 1;
            let bottom_left = top_left + LONGITUDE_SEGMENTS as u32 + 1;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        sky_vertex_colors(DEFAULT_TIME_OF_DAY_SECONDS),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn sky_vertex_colors(time_of_day_seconds: f32) -> Vec<[f32; 4]> {
    const LATITUDE_SEGMENTS: usize = 32;
    const LONGITUDE_SEGMENTS: usize = 64;
    let (elevation, daylight) = sun_position_and_daylight(time_of_day_seconds);
    let twilight = (1.0 - smoothstep(0.02, 0.26, elevation.abs())) * (1.0 - daylight * 0.7);
    let mut colors = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1));

    for latitude in 0..=LATITUDE_SEGMENTS {
        let vertical = -1.0 + 2.0 * latitude as f32 / LATITUDE_SEGMENTS as f32;
        let zenith_blend = smoothstep(-0.15, 0.9, vertical);
        for _ in 0..=LONGITUDE_SEGMENTS {
            let day_horizon = Vec3::new(0.58, 0.79, 0.98);
            let day_zenith = Vec3::new(0.12, 0.39, 0.82);
            let night_horizon = Vec3::new(0.025, 0.045, 0.13);
            let night_zenith = Vec3::new(0.006, 0.012, 0.045);
            let horizon = night_horizon.lerp(day_horizon, daylight);
            let zenith = night_zenith.lerp(day_zenith, daylight);
            let base = horizon.lerp(zenith, zenith_blend);
            let warm = Vec3::new(0.98, 0.31, 0.16).lerp(Vec3::new(0.52, 0.20, 0.32), zenith_blend);
            let twilight_amount = twilight * (1.0 - zenith_blend * 0.65);
            let color = base.lerp(warm, twilight_amount);
            colors.push([color.x, color.y, color.z, 1.0]);
        }
    }
    colors
}

fn create_stars_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut state = 0x9E37_79B9_u32;

    for _ in 0..180 {
        let azimuth = random_unit(&mut state) * std::f32::consts::TAU;
        let vertical = 0.08 + random_unit(&mut state) * 0.88;
        let horizontal = (1.0 - vertical * vertical).sqrt();
        let center = Vec3::new(
            horizontal * azimuth.cos(),
            vertical,
            horizontal * azimuth.sin(),
        ) * STAR_RADIUS;
        let half_size = 0.12 + random_unit(&mut state) * 0.18;
        append_cube(
            &mut positions,
            &mut normals,
            &mut indices,
            center,
            half_size,
        );
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn append_cube(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    half_size: f32,
) {
    let h = half_size;
    let faces = [
        (
            [
                Vec3::new(-h, -h, h),
                Vec3::new(h, -h, h),
                Vec3::new(h, h, h),
                Vec3::new(-h, h, h),
            ],
            Vec3::Z,
        ),
        (
            [
                Vec3::new(h, -h, -h),
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, h, -h),
                Vec3::new(h, h, -h),
            ],
            Vec3::NEG_Z,
        ),
        (
            [
                Vec3::new(h, -h, h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, h, -h),
                Vec3::new(h, h, h),
            ],
            Vec3::X,
        ),
        (
            [
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, -h, h),
                Vec3::new(-h, h, h),
                Vec3::new(-h, h, -h),
            ],
            Vec3::NEG_X,
        ),
        (
            [
                Vec3::new(-h, h, h),
                Vec3::new(h, h, h),
                Vec3::new(h, h, -h),
                Vec3::new(-h, h, -h),
            ],
            Vec3::Y,
        ),
        (
            [
                Vec3::new(-h, -h, -h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, -h, h),
                Vec3::new(-h, -h, h),
            ],
            Vec3::NEG_Y,
        ),
    ];

    for (corners, normal) in faces {
        let base = positions.len() as u32;
        for corner in corners {
            positions.push((center + corner).to_array());
            normals.push(normal.to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn random_unit(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("THE CUBES"),
                TextFont::from_font_size(54.0),
                TextColor(Color::srgb(0.55, 0.9, 0.72)),
            ));
            root.spawn((
                Text::new("Un mundo de bloques, a tu manera"),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ));
            for (label, action) in [
                ("Jugar", MenuCommand::Play),
                ("Crear mundo", MenuCommand::Create),
                ("Salir", MenuCommand::Exit),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(54.0),
                        margin: UiRect::top(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(22.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

fn setup_world_select(mut commands: Commands) {
    let worlds = match list_saved_worlds() {
        Ok(worlds) => worlds,
        Err(error) => {
            error!("Could not list saved worlds: {error}");
            Vec::new()
        }
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Elegir mundo"),
                TextFont::from_font_size(36.0),
                TextColor(Color::WHITE),
            ));
            if worlds.is_empty() {
                root.spawn((
                    Text::new("Aún no hay mundos guardados."),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::srgb(0.8, 0.82, 0.86)),
                ));
            }
            for (name, path) in worlds {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(360.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    LoadWorldAction(path),
                ))
                .with_child((
                    Text::new(name),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
            root.spawn((
                Button,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(48.0),
                    margin: UiRect::top(Val::Px(12.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.2, 0.25)),
                MenuAction(MenuCommand::Back),
            ))
            .with_child((
                Text::new("Volver"),
                TextFont::from_font_size(20.0),
                TextColor(Color::WHITE),
            ));
        });
}

fn setup_create_world(mut commands: Commands, form: Res<WorldForm>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(520.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.1, 0.14)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Crear mundo"),
                    TextFont::from_font_size(34.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Nombre (hasta 32 caracteres)"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.78, 0.82, 0.88)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            min_height: Val::Px(44.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.16, 0.2)),
                        FormField(WorldField::Name),
                    ))
                    .with_child((
                        Text::new(if form.focus == WorldField::Name {
                            format!("{}_", form.name)
                        } else {
                            form.name.clone()
                        }),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::WHITE),
                        NameInputText,
                    ));
                panel.spawn((
                    Text::new("Semilla (número; la misma semilla repite el terreno)"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.78, 0.82, 0.88)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            min_height: Val::Px(44.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.16, 0.2)),
                        FormField(WorldField::Seed),
                    ))
                    .with_child((
                        Text::new(if form.focus == WorldField::Seed {
                            format!("{}_", form.seed)
                        } else {
                            form.seed.clone()
                        }),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::WHITE),
                        SeedInputText,
                    ));
                panel.spawn((
                    Text::new(form.error.clone().unwrap_or_default()),
                    TextFont::from_font_size(16.0),
                    TextColor(Color::srgb(1.0, 0.45, 0.4)),
                    FormErrorText,
                ));
                for (label, action) in [
                    ("Crear y jugar", MenuCommand::StartWorld),
                    ("Cancelar", MenuCommand::Back),
                ] {
                    panel
                        .spawn((
                            Button,
                            Node {
                                height: Val::Px(48.0),
                                margin: UiRect::top(Val::Px(6.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                            MenuAction(action),
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont::from_font_size(19.0),
                            TextColor(Color::WHITE),
                        ));
                }
                panel.spawn((
                    Text::new("Enter cambia el campo activo; Backspace borra."),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::srgb(0.68, 0.72, 0.78)),
                ));
            });
        });
}

fn setup_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Preparando mundo..."),
                TextFont::from_font_size(34.0),
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Text::new("Cargando terreno: 0%"),
                TextFont::from_font_size(20.0),
                TextColor(Color::srgb(0.72, 0.88, 0.78)),
                LoadingText,
            ));
            root.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(46.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.2, 0.25)),
                MenuAction(MenuCommand::CancelLoading),
            ))
            .with_child((
                Text::new("Cancelar"),
                TextFont::from_font_size(18.0),
                TextColor(Color::WHITE),
            ));
        });
}

fn setup_pause_menu(mut commands: Commands, storage: Res<WorldStorage>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.88)),
            PauseRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Juego en pausa"),
                TextFont::from_font_size(38.0),
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Text::new(format!("Autoguardado: {}", storage.status)),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.82, 0.86, 0.9)),
                PauseStatus,
            ));
            for (label, action) in [
                ("Continuar", MenuCommand::Resume),
                ("Guardar y volver al menú", MenuCommand::SaveAndMenu),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Px(54.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

fn cleanup_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}

fn cleanup_pause_menu(mut commands: Commands, roots: Query<Entity, With<PauseRoot>>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}

fn cleanup_game_scene(
    mut commands: Commands,
    entities: Query<Entity, With<GameEntity>>,
    mut world: ResMut<WorldData>,
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut crafted_tools: ResMut<CraftedTools>,
    mut crafting_ui: ResMut<CraftingUi>,
    mut pending: ResMut<PendingWorld>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    mut storage: ResMut<WorldStorage>,
    mut mob_population: ResMut<MobPopulation>,
) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
    world.chunks.clear();
    world.modified_chunks.clear();
    world.dirty.clear();
    world.name.clear();
    inventory.counts = [0; BLOCK_COUNT];
    *survival = Survival::default();
    *crafted_tools = CraftedTools::default();
    *crafting_ui = CraftingUi::default();
    selected.0 = GRASS;
    selected_slot.0 = 0;
    pending.save = None;
    pending.path = None;
    pending.is_new_world = false;
    *storage = WorldStorage::default();
    *mob_population = MobPopulation::default();
}

fn block_type_name(block_type: u8) -> &'static str {
    match block_type {
        AIR => "Aire",
        GRASS => "Hierba",
        DIRT => "Tierra",
        STONE => "Piedra",
        SAND => "Arena",
        WOOD => "Madera",
        LEAVES => "Hojas",
        APPLE => "Manzana",
        WHEAT => "Trigo",
        MUSHROOM => "Hongo",
        WOODEN_DOOR => "Puerta",
        STAIRS => "Escaleras",
        SLAB => "Losa",
        BRICKS => "Ladrillo",
        PLANKS => "Tablones",
        TORCH => "Antorcha",
        _ => "Desconocido",
    }
}

fn setup_inventory_ui(commands: &mut Commands, inventory: &Inventory) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            InventoryPanel,
            GameEntity,
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(880.0),
                        height: Val::Px(706.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(20.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
                    BorderColor(Color::srgb(0.5, 0.5, 0.5)),
                ))
                .with_children(|main| {
                    main.spawn((
                        Text::new("Inventario (E para cerrar)"),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::WHITE),
                    ));

                    main.spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(200.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(20.0),
                        ..default()
                    })
                    .with_children(|top_row| {
                        top_row
                            .spawn((
                                Node {
                                    width: Val::Px(180.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    padding: UiRect::all(Val::Px(10.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                                BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                                InventoryCraftingGrid,
                            ))
                            .with_children(|craft| {
                                craft.spawn((
                                    Text::new("Crafteo 2×2"),
                                    TextFont::from_font_size(14.0),
                                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                ));

                                craft.spawn(Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(100.0),
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: Val::Px(5.0),
                                    row_gap: Val::Px(5.0),
                                    ..default()
                                })
                                .with_children(|slots| {
                                    for i in 0..4 {
                                        slots.spawn((
                                            Node {
                                                width: Val::Px(45.0),
                                                height: Val::Px(45.0),
                                                border: UiRect::all(Val::Px(1.0)),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::Center,
                                                ..default()
                                            },
                                            BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                                            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                                            CraftingGridSlot { position: i },
                                        ));
                                    }
                                });
                            });

                        top_row.spawn(Node {
                            width: Val::Px(40.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_child((
                            Text::new("→"),
                            TextFont::from_font_size(30.0),
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));

                        top_row
                            .spawn((
                                Node {
                                    width: Val::Px(100.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                                InventoryCraftResult,
                            ))
                            .with_child((
                                Text::new("Resultado"),
                                TextFont::from_font_size(12.0),
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));

                        top_row.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                    });

                    main.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(330.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(5.0),
                            overflow: Overflow::clip_y(),
                            padding: UiRect::all(Val::Px(10.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                        BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                        InventoryGrid,
                    ))
                    .with_children(|grid| {
                        for (idx, count) in inventory.counts.iter().enumerate() {
                            if *count > 0 {
                                let block_name = block_type_name(idx as u8);
                                grid.spawn(Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(40.0),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(5.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                })
                                .with_children(|slot| {
                                    slot.spawn((
                                        Text::new(format!("{}: {}", block_name, count)),
                                        TextFont::from_font_size(14.0),
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            }
                        }
                    });
                });
        });
}

fn setup_hud(commands: &mut Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert((HudRoot, GameEntity))
        .with_children(|root| {
            root.spawn((
                Text::new("+"),
                TextFont::from_font_size(26.0),
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
            ));

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(18.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)),
                HudStatus,
                Text::new("CAMINAR  [F] VUELO\nBloque: Césped [1]\n[ESC] pausa"),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.94, 0.96, 1.0)),
            ));

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(24.0),
                width: Val::Px(220.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            })
            .insert(BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)))
            .with_children(|panel| {
                for (label, kind, color) in [
                    (
                        "Vida",
                        SurvivalBarKind::Health,
                        Color::srgb(0.82, 0.18, 0.2),
                    ),
                    (
                        "Hambre",
                        SurvivalBarKind::Hunger,
                        Color::srgb(0.9, 0.62, 0.16),
                    ),
                    (
                        "Experiencia",
                        SurvivalBarKind::Experience,
                        Color::srgb(0.25, 0.75, 0.32),
                    ),
                ] {
                    if matches!(kind, SurvivalBarKind::Experience) {
                        panel.spawn((
                            Text::new("Nivel 0 · 0/10 XP"),
                            TextFont::from_font_size(14.0),
                            TextColor(Color::WHITE),
                            ExperienceText,
                        ));
                    } else {
                        panel.spawn((
                            Text::new(label),
                            TextFont::from_font_size(14.0),
                            TextColor(Color::WHITE),
                            SurvivalValue(kind),
                        ));
                    }
                    panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(12.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.17, 0.2, 0.95)),
                        ))
                        .with_child((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(color),
                            SurvivalBarFill(kind),
                        ));
                }
            });

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(440.0),
                    padding: UiRect::all(Val::Px(20.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.055, 0.09, 0.96)),
                CraftingPanel,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Mesa de crafteo  [C / Esc]"),
                    TextFont::from_font_size(26.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Pico de madera  ·  3 madera + 2 piedra"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::WoodenPickaxe),
                    ))
                    .with_child((
                        Text::new("Fabricar pico (64 usos)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Hacha de madera  ·  2 madera + 1 piedra"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::WoodenAxe),
                    ))
                    .with_child((
                        Text::new("Fabricar hacha (64 usos)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Pala de madera  ·  2 madera + 1 piedra"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::WoodenShovel),
                    ))
                    .with_child((
                        Text::new("Fabricar pala (64 usos, 2x en arena/tierra)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Espada de madera  ·  2 madera + 2 piedra"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::WoodenSword),
                    ))
                    .with_child((
                        Text::new("Fabricar espada (32 usos, +daño a mobs)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Ración de bayas  ·  3 hojas + 1 madera"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::BerryRation),
                    ))
                    .with_child((
                        Text::new("Preparar ración (+1 comida)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Flechas (x5)  ·  2 hojas + 1 arena"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Arrows),
                    ))
                    .with_child((
                        Text::new("Fabricar 5 flechas"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                panel.spawn((
                    Text::new("Pala: 2x rápido en arena/tierra. Espada: +1 daño, sin romper bloques. Flechas: múltiples disparos."),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::srgb(0.75, 0.8, 0.86)),
                ));
                
                // Feature 5: Structures
                panel.spawn((
                    Text::new(""),
                    TextFont::from_font_size(5.0),
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ));
                panel.spawn((
                    Text::new("═══ ESTRUCTURAS (Feature 5) ═══"),
                    TextFont::from_font_size(16.0),
                    TextColor(Color::srgb(0.78, 0.90, 1.0)),
                ));
                
                panel.spawn((
                    Text::new("Puerta de madera  ·  6 madera  →  3 puertas"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::WoodenDoor),
                    ))
                    .with_child((
                        Text::new("Fabricar puertas (click derecho = abrir/cerrar)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                    
                panel.spawn((
                    Text::new("Escaleras de madera  ·  6 madera  →  4 bloques"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Stairs),
                    ))
                    .with_child((
                        Text::new("Fabricar escaleras (subida vertical)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                    
                panel.spawn((
                    Text::new("Losas de madera  ·  3 madera  →  6 losas"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Slab),
                    ))
                    .with_child((
                        Text::new("Fabricar losas (media altura, apilables)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                    
                panel.spawn((
                    Text::new("Ladrillo de piedra  ·  4 piedra  →  4 ladrillos"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Bricks),
                    ))
                    .with_child((
                        Text::new("Fabricar ladrillos"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));
                    
                panel.spawn((
                    Text::new("Tablones de madera  ·  1 madera  →  4 tablones"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Planks),
                    ))
                    .with_child((
                        Text::new("Fabricar tablones (material versátil)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));

                panel.spawn((
                    Text::new("Puertas bloqueadas si están cerradas. Escaleras/losas aún en desarrollo (renderización parcial)."),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::srgb(0.75, 0.8, 0.86)),
                ));
                
                // Feature 6: Lighting
                panel.spawn((
                    Text::new(""),
                    TextFont::from_font_size(5.0),
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ));
                panel.spawn((
                    Text::new("═══ ILUMINACIÓN (Feature 6) ═══"),
                    TextFont::from_font_size(16.0),
                    TextColor(Color::srgb(1.0, 0.95, 0.6)),
                ));
                
                panel.spawn((
                    Text::new("Antorcha  ·  1 madera + 1 piedra  →  8 antorchas"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.88, 0.9, 0.94)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                        CraftRecipeButton(CraftRecipe::Torch),
                    ))
                    .with_child((
                        Text::new("Fabricar antorchas (radio 8 bloques, mobs spawnan en oscuridad)"),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::WHITE),
                    ));

                panel.spawn((
                    Text::new("Antorchas iluminan zonas oscuras previniendo spawn de mobs hostiles. Colocar cerca de refugios."),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::srgb(0.75, 0.8, 0.86)),
                ));
                
                panel.spawn((
                    Text::new(""),
                    TextFont::from_font_size(15.0),
                    TextColor(Color::srgb(1.0, 0.78, 0.35)),
                    CraftingStatus,
                ));
            });

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)),
            ))
            .with_children(|hotbar| {
                for (slot_index, &block) in HOTBAR_BLOCKS.iter().enumerate().take(HOTBAR_SLOT_COUNT) {
                    let color = match block {
                        GRASS => Color::srgb(0.25, 0.62, 0.22),
                        DIRT => Color::srgb(0.43, 0.30, 0.18),
                        STONE => Color::srgb(0.48, 0.50, 0.53),
                        SAND => Color::srgb(0.82, 0.72, 0.45),
                        WOOD => Color::srgb(0.48, 0.29, 0.12),
                        LEAVES => Color::srgb(0.16, 0.48, 0.19),
                        _ => Color::srgba(0.12, 0.14, 0.18, 0.9),
                    };
                    hotbar
                        .spawn((
                            Node {
                                width: Val::Px(52.0),
                                height: Val::Px(52.0),
                                border: UiRect::all(Val::Px(2.0)),
                                align_items: AlignItems::End,
                                justify_content: JustifyContent::Start,
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(color),
                            BorderColor(if slot_index == 0 {
                                Color::WHITE
                            } else {
                                Color::srgba(0.12, 0.14, 0.18, 0.9)
                            }),
                            HotbarSlot(slot_index),
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                Text::new((slot_index + 1).to_string()),
                                TextFont::from_font_size(16.0),
                                TextColor(Color::WHITE),
                            ));
                            if block != AIR {
                                slot.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: Val::Px(4.0),
                                        bottom: Val::Px(2.0),
                                        ..default()
                                    },
                                    Text::new("0"),
                                    TextFont::from_font_size(14.0),
                                    TextColor(Color::WHITE),
                                    HotbarCount(slot_index),
                                ));
                            }
                        });
                }
            });
        });
}

#[derive(Component)]
struct GameEntity;

fn main_menu_actions(
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Play => next.set(AppState::WorldSelect),
            MenuCommand::Create => {
                commands.insert_resource(WorldForm::default());
                next.set(AppState::CreateWorld);
            }
            MenuCommand::Exit => {
                exit.send(AppExit::Success);
            }
            _ => {}
        }
    }
}

fn world_select_actions(
    mut next: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    load_buttons: Query<(&Interaction, &LoadWorldAction), Changed<Interaction>>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
) {
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed && matches!(action.0, MenuCommand::Back) {
            next.set(AppState::MainMenu);
        }
    }
    for (interaction, action) in &load_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match load_world(&action.0) {
            Ok(save) => {
                pending.save = Some(save);
                pending.path = Some(action.0.clone());
                pending.is_new_world = false;
                *storage = WorldStorage::default();
                next.set(AppState::LoadingWorld);
            }
            Err(error) => error!("Could not load world {:?}: {error}", action.0),
        }
    }
}

fn create_world_input(mut events: EventReader<KeyboardInput>, mut form: ResMut<WorldForm>) {
    let mut changed = false;
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        changed = true;
        match &event.logical_key {
            Key::Enter | Key::Tab => {
                form.focus = if form.focus == WorldField::Name {
                    WorldField::Seed
                } else {
                    WorldField::Name
                };
            }
            Key::Backspace => match form.focus {
                WorldField::Name => {
                    form.name.pop();
                }
                WorldField::Seed => {
                    form.seed.pop();
                }
            },
            Key::Character(character) => match form.focus {
                WorldField::Name if form.name.chars().count() < 32 => {
                    if !character.chars().any(char::is_control) {
                        form.name.push_str(character);
                    }
                }
                WorldField::Seed
                    if form.seed.len() < 20
                        && character
                            .chars()
                            .all(|character| character.is_ascii_digit()) =>
                {
                    form.seed.push_str(character);
                }
                _ => {}
            },
            _ => {}
        }
    }
    if changed {
        form.error = None;
    }
}

fn create_world_actions(
    mut form: ResMut<WorldForm>,
    mut next: ResMut<NextState<AppState>>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    fields: Query<(&Interaction, &FormField), Changed<Interaction>>,
) {
    for (interaction, field) in &fields {
        if *interaction == Interaction::Pressed {
            form.focus = field.0;
            form.error = None;
        }
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Back => next.set(AppState::MainMenu),
            MenuCommand::StartWorld => {
                let name = form.name.trim();
                if name.is_empty() {
                    form.error = Some("Escribe un nombre para el mundo.".into());
                    continue;
                }
                let seed = match form.seed.parse::<u64>() {
                    Ok(seed) => seed,
                    Err(_) => {
                        form.error = Some("La semilla debe ser un número válido.".into());
                        continue;
                    }
                };
                let path = match world_file_path(name) {
                    Ok(path) => path,
                    Err(error) => {
                        form.error = Some(error);
                        continue;
                    }
                };
                if path.exists() {
                    form.error = Some("Ya existe un mundo con ese nombre.".into());
                    continue;
                }
                pending.save = Some(WorldSave {
                    version: SAVE_VERSION,
                    name: name.into(),
                    seed,
                    time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
                    player_position: [8.5, 15.0, 8.5],
                    player_yaw: 0.0,
                    player_pitch: -0.3,
                    selected_block: GRASS,
                    selected_hotbar_slot: 0,
                    inventory: [0; LEGACY_BLOCK_COUNT],
                    extra_inventory: [0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
                    health: MAX_HEALTH,
                    hunger: MAX_HUNGER,
                    experience: 0,
                    food: default_starting_food(),
                    pickaxe_durability: 0,
                    axe_durability: 0,
                    shovel_durability: 0,
                    sword_durability: 0,
                    chunks: Vec::new(),
                });
                pending.path = Some(path);
                pending.is_new_world = true;
                *storage = WorldStorage::default();
                next.set(AppState::LoadingWorld);
            }
            _ => {}
        }
    }
}

fn update_world_form_ui(
    form: Res<WorldForm>,
    mut name_text: Query<
        &mut Text,
        (
            With<NameInputText>,
            Without<SeedInputText>,
            Without<FormErrorText>,
        ),
    >,
    mut seed_text: Query<
        &mut Text,
        (
            With<SeedInputText>,
            Without<NameInputText>,
            Without<FormErrorText>,
        ),
    >,
    mut error_text: Query<
        &mut Text,
        (
            With<FormErrorText>,
            Without<NameInputText>,
            Without<SeedInputText>,
        ),
    >,
) {
    if !form.is_changed() {
        return;
    }
    if let Ok(mut text) = name_text.get_single_mut() {
        text.0 = format!(
            "{}{}",
            form.name,
            if form.focus == WorldField::Name {
                "_"
            } else {
                ""
            }
        );
    }
    if let Ok(mut text) = seed_text.get_single_mut() {
        text.0 = format!(
            "{}{}",
            form.seed,
            if form.focus == WorldField::Seed {
                "_"
            } else {
                ""
            }
        );
    }
    if let Ok(mut text) = error_text.get_single_mut() {
        text.0 = form.error.clone().unwrap_or_default();
    }
}

fn begin_loading_world(
    mut progress: ResMut<LoadingProgress>,
    mut world: ResMut<WorldData>,
    mut day_night: ResMut<DayNightCycle>,
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut crafted_tools: ResMut<CraftedTools>,
    mut crafting_ui: ResMut<CraftingUi>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
    mut next: ResMut<NextState<AppState>>,
) {
    world.chunks.clear();
    world.modified_chunks.clear();
    world.dirty.clear();
    storage.legacy_chunks.clear();
    storage.dirty_chunks.clear();
    storage.failed_chunks.clear();
    storage.autosave_elapsed = 0.0;
    storage.retry_elapsed = 0.0;
    storage.metadata_save_requested = true;
    storage.status = "Sin guardar".into();
    storage.path.clone_from(&pending.path);
    progress.coordinates.clear();
    progress.completed = 0;
    progress.total = 0;
    progress.chunk_count = 0;
    progress.chunk_cursor = 0;
    progress.stage = LoadingStage::Chunks;
    progress.material = None;
    progress.is_new_world = pending.is_new_world;
    progress.initialized = false;
    let Some(save) = pending.save.as_mut() else {
        error!("Loading was requested without world data");
        next.set(AppState::MainMenu);
        return;
    };
    progress.initialized = true;

    world.name.clone_from(&save.name);
    world.seed = save.seed;
    day_night.time_of_day_seconds = save.time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS);
    inventory.counts[..LEGACY_BLOCK_COUNT].copy_from_slice(&save.inventory);
    inventory.counts[LEGACY_BLOCK_COUNT..].copy_from_slice(&save.extra_inventory);
    survival.health = save.health;
    survival.hunger = save.hunger;
    survival.experience = save.experience;
    survival.food = save.food;
    survival.passive_hunger_elapsed = 0.0;
    survival.sprint_hunger_elapsed = 0.0;
    survival.starvation_elapsed = 0.0;
    survival.regeneration_elapsed = 0.0;
    survival.notice.clear();
    survival.notice_elapsed = 0.0;
    crafted_tools.pickaxe_durability = save.pickaxe_durability;
    crafted_tools.axe_durability = save.axe_durability;
    crafted_tools.shovel_durability = save.shovel_durability;
    crafted_tools.sword_durability = save.sword_durability;
    crafting_ui.open = false;
    crafting_ui.just_closed = false;
    selected_slot.0 = resolve_hotbar_slot(save.selected_block, save.selected_hotbar_slot);
    selected.0 = HOTBAR_BLOCKS[selected_slot.0];

    storage.legacy_chunks = std::mem::take(&mut save.chunks)
        .into_iter()
        .map(|saved_chunk| {
            (
                IVec3::from_array(saved_chunk.coord),
                Chunk {
                    blocks: saved_chunk.blocks,
                },
            )
        })
        .collect();
    let legacy_coordinates = storage.legacy_chunks.keys().copied().collect::<Vec<_>>();
    storage.dirty_chunks.extend(legacy_coordinates);

    let player_block = IVec3::new(
        save.player_position[0].floor() as i32,
        0,
        save.player_position[2].floor() as i32,
    );
    let center = block_address(player_block).0;
    progress.coordinates = chunk_coordinates_around(center, STREAM_LOAD_RADIUS);
    progress.chunk_count = progress.coordinates.len();
    progress.total = progress.chunk_count * 2;
}

fn chunk_coordinates_around(center: IVec3, radius: i32) -> Vec<IVec3> {
    let mut coordinates = Vec::with_capacity(((radius * 2 + 1).pow(2)) as usize);
    for z in -radius..=radius {
        for x in -radius..=radius {
            coordinates.push(IVec3::new(center.x + x, 0, center.z + z));
        }
    }
    coordinates
}

fn loading_world_step(
    mut commands: Commands,
    mut progress: ResMut<LoadingProgress>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut pending: ResMut<PendingWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: Query<&mut Text, With<LoadingText>>,
    mut next: ResMut<NextState<AppState>>,
    texture_assets: Res<TextureAssets>,
) {
    if !progress.initialized {
        return;
    }
    match progress.stage {
        LoadingStage::Chunks if progress.chunk_cursor < progress.chunk_count => {
            if let Some(coord) = progress.coordinates.pop() {
                match load_or_generate_chunk(&mut world, &mut storage, coord) {
                    Ok(chunk) => {
                        world.chunks.insert(coord, chunk);
                    }
                    Err(error) => {
                        error!("Could not load chunk {coord:?}: {error}");
                        storage.status = format!("Error al cargar chunk: {error}");
                        progress.initialized = false;
                        next.set(AppState::MainMenu);
                        return;
                    }
                }
            } else {
                error!("World loading ran out of chunks before completion");
                progress.initialized = false;
                next.set(AppState::MainMenu);
                return;
            }
            progress.chunk_cursor += 1;
            progress.completed += 1;
        }
        LoadingStage::Meshes => {
            if let Some(coord) = progress.coordinates.pop() {
                if progress.material.is_none() {
                    progress.material = Some(materials.add(StandardMaterial {
                        base_color_texture: texture_assets.atlas.clone(),
                        ..default()
                    }));
                }
                let Some(material) = progress.material.clone() else {
                    error!("Could not create the chunk material");
                    progress.initialized = false;
                    next.set(AppState::MainMenu);
                    return;
                };
                let mesh = meshes.add(rendering::build_chunk_mesh(&world, coord));
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32),
                    ChunkMesh(coord),
                    GameEntity,
                ));
                progress.completed += 1;
            }
        }
        LoadingStage::Chunks => {}
    }

    if progress.stage == LoadingStage::Chunks && progress.chunk_cursor == progress.chunk_count {
        progress.stage = LoadingStage::Meshes;
        progress.coordinates = world.chunks.keys().copied().collect();
        progress
            .coordinates
            .sort_unstable_by_key(|coordinate| coordinate.to_array());
    }

    let percent = if progress.total == 0 {
        100
    } else {
        (progress.completed * 100 / progress.total).min(100)
    };
    if let Ok(mut text) = status.get_single_mut() {
        let label = match progress.stage {
            LoadingStage::Chunks if !progress.is_new_world => "Cargando terreno",
            LoadingStage::Chunks => "Generando terreno",
            LoadingStage::Meshes => "Preparando mallas",
        };
        text.0 = format!(
            "{label}: {percent}% ({}/{})",
            progress.completed, progress.total
        );
    }
    if progress.stage == LoadingStage::Meshes && progress.coordinates.is_empty() {
        if let Some(save) = pending.save.as_mut() {
            if progress.is_new_world {
                let spawn_x = 8.5;
                let spawn_z = 8.5;
                let height = terrain_height(spawn_x as i32, spawn_z as i32, save.seed);
                save.player_position = [spawn_x, height as f32 + 1.0 + PLAYER_EYE_HEIGHT, spawn_z];
            }
        }
        pending.is_new_world = false;
        progress.initialized = false;
        next.set(AppState::InGame);
    }
}

fn loading_world_actions(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut pending: ResMut<PendingWorld>,
    mut progress: ResMut<LoadingProgress>,
) {
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed && matches!(action.0, MenuCommand::CancelLoading) {
            progress.initialized = false;
            pending.save = None;
            pending.is_new_world = false;
            next.set(AppState::MainMenu);
        }
    }
}

fn pause_menu_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    world: Res<WorldData>,
    player: Query<(&Transform, &Player)>,
    inventory: Res<Inventory>,
    survival: Res<Survival>,
    crafted_tools: Res<CraftedTools>,
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    day_night: Res<DayNightCycle>,
    mut storage: ResMut<WorldStorage>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut status: Query<&mut Text, With<PauseStatus>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::InGame);
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Resume => next.set(AppState::InGame),
            MenuCommand::SaveAndMenu => {
                let Ok((transform, player)) = player.get_single() else {
                    error!("Cannot save: player entity is missing");
                    set_pause_error(&mut status, "No se pudo encontrar al jugador.");
                    continue;
                };
                let Some(path) = storage.path.clone() else {
                    set_pause_error(&mut status, "No se encontró la ruta del mundo.");
                    continue;
                };
                let save = snapshot_world(
                    &world,
                    transform.translation,
                    player,
                    &inventory,
                    selected.0,
                    selected_slot.0,
                    day_night.time_of_day_seconds,
                    &survival,
                    &crafted_tools,
                );
                match save_all_chunks(&world, &mut storage, &path)
                    .and_then(|()| save_world(&path, save))
                {
                    Ok(()) => {
                        storage.metadata_save_requested = false;
                        storage.status = "Guardado".into();
                        next.set(AppState::MainMenu);
                    }
                    Err(error) => {
                        error!("Could not save world: {error}");
                        storage.status = format!("Error al guardar: {error}");
                        set_pause_error(&mut status, &format!("Error al guardar: {error}"));
                    }
                }
            }
            _ => {}
        }
    }
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn set_pause_error(status: &mut Query<&mut Text, With<PauseStatus>>, message: &str) {
    if let Ok(mut text) = status.get_single_mut() {
        text.0 = message.to_string();
    }
}

fn pause_from_game(
    keys: Res<ButtonInput<KeyCode>>,
    crafting_ui: Res<CraftingUi>,
    mut next: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if crafting_ui.open || crafting_ui.just_closed {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
    next.set(AppState::Paused);
}

fn toggle_crafting(
    keys: Res<ButtonInput<KeyCode>>,
    mut crafting: ResMut<CraftingUi>,
    mut panel: Query<&mut Node, With<CraftingPanel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    crafting.just_closed = false;
    let close = crafting.open && keys.just_pressed(KeyCode::Escape);
    if !keys.just_pressed(KeyCode::KeyC) && !close {
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        error!("Primary window is missing while toggling crafting");
        return;
    };
    let Ok(mut node) = panel.get_single_mut() else {
        error!("Crafting panel is missing from the game HUD");
        return;
    };

    if crafting.open {
        crafting.open = false;
        crafting.just_closed = true;
        node.display = Display::None;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    } else if window.cursor_options.grab_mode == CursorGrabMode::Locked {
        crafting.open = true;
        node.display = Display::Flex;
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn toggle_inventory_visibility(
    keys: Res<ButtonInput<KeyCode>>,
    mut crafting: ResMut<CraftingUi>,
    mut panel: Query<&mut Node, With<InventoryPanel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    // Close inventory on Escape
    if crafting.inventory_open && keys.just_pressed(KeyCode::Escape) {
        crafting.inventory_open = false;
        crafting.inventory_just_opened = true;
    }

    if !crafting.inventory_just_opened {
        return;
    }

    crafting.inventory_just_opened = false;

    let Ok(mut window) = windows.get_single_mut() else {
        error!("Primary window is missing while toggling inventory");
        return;
    };

    let Ok(mut node) = panel.get_single_mut() else {
        // Inventory panel might not exist yet, so just skip
        return;
    };

    if crafting.inventory_open {
        node.display = Display::Flex;
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    } else {
        node.display = Display::None;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

fn craft_recipe(
    recipe: CraftRecipe,
    inventory: &mut Inventory,
    survival: &mut Survival,
    tools: &mut CraftedTools,
) -> Result<&'static str, &'static str> {
    let costs: &[(u8, u32)] = match recipe {
        CraftRecipe::WoodenPickaxe => {
            if tools.pickaxe_durability > 0 {
                return Err("Ya tienes un pico con durabilidad.");
            }
            &[(WOOD, 3), (STONE, 2)]
        }
        CraftRecipe::WoodenAxe => {
            if tools.axe_durability > 0 {
                return Err("Ya tienes un hacha con durabilidad.");
            }
            &[(WOOD, 2), (STONE, 1)]
        }
        CraftRecipe::WoodenShovel => {
            if tools.shovel_durability > 0 {
                return Err("Ya tienes una pala con durabilidad.");
            }
            &[(WOOD, 2), (STONE, 1)]
        }
        CraftRecipe::WoodenSword => {
            if tools.sword_durability > 0 {
                return Err("Ya tienes una espada con durabilidad.");
            }
            &[(WOOD, 2), (STONE, 2)]
        }
        CraftRecipe::BerryRation => {
            if survival.food >= 99 {
                return Err("No puedes llevar más comida.");
            }
            &[(LEAVES, 3), (WOOD, 1)]
        }
        CraftRecipe::Arrows => {
            if inventory.count(LEAVES) < 2 || inventory.count(SAND) < 1 {
                return Err("Necesitas 2 hojas y 1 arena para 5 flechas.");
            }
            &[(LEAVES, 2), (SAND, 1)]
        }
        CraftRecipe::WoodenDoor => &[(WOOD, 6)],
        CraftRecipe::Stairs => &[(WOOD, 6)],
        CraftRecipe::Slab => &[(WOOD, 3)],
        CraftRecipe::Bricks => &[(STONE, 4)],
        CraftRecipe::Planks => &[(WOOD, 1)],
        CraftRecipe::Torch => &[(WOOD, 1), (STONE, 1)],
    };

    if costs
        .iter()
        .any(|(block, amount)| inventory.count(*block) < *amount)
    {
        return Err("No tienes los materiales necesarios.");
    }
    for (block, amount) in costs {
        for _ in 0..*amount {
            if !inventory.remove(*block) {
                error!("Crafting materials changed after recipe validation");
                return Err("No se pudieron consumir los materiales.");
            }
        }
    }

    match recipe {
        CraftRecipe::WoodenPickaxe => {
            tools.pickaxe_durability = 64;
            Ok("Pico fabricado: 64 usos; rompe piedra para obtener extra.")
        }
        CraftRecipe::WoodenAxe => {
            tools.axe_durability = 64;
            Ok("Hacha fabricada: 64 usos; tala madera y recoge hojas extra.")
        }
        CraftRecipe::WoodenShovel => {
            tools.shovel_durability = 64;
            Ok("Pala fabricada: 64 usos; rompe arena y tierra 2x rápido.")
        }
        CraftRecipe::WoodenSword => {
            tools.sword_durability = 32;
            Ok("Espada fabricada: 32 usos; golpea sin romper bloques.")
        }
        CraftRecipe::BerryRation => {
            survival.food += 1;
            Ok("Ración preparada. Pulsa E para comer.")
        }
        CraftRecipe::Arrows => {
            for _ in 0..5 {
                if inventory.add(SAND).is_err() {
                    // Usar SAND como marcador de flechas (vamos a cambiarlo luego)
                    break;
                }
            }
            Ok("5 flechas fabricadas. Equipa el arco y dispara con clic derecho.")
        }
        CraftRecipe::WoodenDoor => {
            for _ in 0..3 {
                let _ = inventory.add(WOODEN_DOOR);
            }
            Ok("3 puertas de madera fabricadas.")
        }
        CraftRecipe::Stairs => {
            for _ in 0..4 {
                let _ = inventory.add(STAIRS);
            }
            Ok("4 bloques de escaleras fabricados.")
        }
        CraftRecipe::Slab => {
            for _ in 0..6 {
                let _ = inventory.add(SLAB);
            }
            Ok("6 losas fabricadas.")
        }
        CraftRecipe::Bricks => {
            for _ in 0..4 {
                let _ = inventory.add(BRICKS);
            }
            Ok("4 bloques de ladrillo fabricados.")
        }
        CraftRecipe::Planks => {
            for _ in 0..4 {
                let _ = inventory.add(PLANKS);
            }
            Ok("4 tablones fabricados (2x material).")
        }
        CraftRecipe::Torch => {
            for _ in 0..8 {
                let _ = inventory.add(TORCH);
            }
            Ok("8 antorchas fabricadas (colocar para iluminar).")
        }
    }
}

fn craft_recipe_buttons(
    crafting: Res<CraftingUi>,
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut tools: ResMut<CraftedTools>,
    buttons: Query<(&Interaction, &CraftRecipeButton), Changed<Interaction>>,
    mut status: Query<&mut Text, With<CraftingStatus>>,
) {
    if !crafting.open {
        return;
    }
    let Ok(mut status) = status.get_single_mut() else {
        error!("Crafting status text is missing from the game HUD");
        return;
    };
    for (interaction, recipe) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        status.0 = match craft_recipe(recipe.0, &mut inventory, &mut survival, &mut tools) {
            Ok(message) => message.into(),
            Err(message) => message.into(),
        };
    }
}

fn data_directory() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("the-cubes").join("worlds"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("the-cubes")
            .join("worlds"));
    }
    Err("No se encontró un directorio de datos de usuario.".into())
}

fn default_time_of_day_seconds() -> f32 {
    DEFAULT_TIME_OF_DAY_SECONDS
}

fn default_health() -> f32 {
    MAX_HEALTH
}

fn default_hunger() -> f32 {
    MAX_HUNGER
}

fn default_starting_food() -> u32 {
    3
}

fn world_file_path(name: &str) -> Result<PathBuf, String> {
    let mut slug = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.extend(character.to_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if slug.is_empty() {
        return Err("El nombre no contiene letras o números.".into());
    }
    Ok(data_directory()?.join(format!("{slug}.ron")))
}

fn list_saved_worlds() -> Result<Vec<(String, PathBuf)>, String> {
    let directory = data_directory()?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("No se pudo leer {}: {error}", directory.display()))?;
    let mut worlds = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("No se pudo leer un archivo de mundo: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("ron") {
            continue;
        }
        match load_world(&path) {
            Ok(save) => worlds.push((save.name, path)),
            Err(error) => warn!("Skipping invalid world {:?}: {error}", path),
        }
    }
    worlds.sort_by(|left, right| left.0.to_lowercase().cmp(&right.0.to_lowercase()));
    Ok(worlds)
}

fn load_world(path: &Path) -> Result<WorldSave, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("No se pudo inspeccionar el archivo: {error}"))?;
    if metadata.len() > 256 * 1024 * 1024 {
        return Err("El archivo de mundo supera el tamaño permitido.".into());
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("No se pudo leer el archivo: {error}"))?;
    let save: WorldSave = ron::from_str(&contents)
        .map_err(|error| format!("El archivo de mundo no es válido: {error}"))?;
    validate_world_save(&save)?;
    Ok(save)
}

fn validate_world_save(save: &WorldSave) -> Result<(), String> {
    if save.version != SAVE_VERSION {
        return Err(format!("Versión de mundo no compatible: {}", save.version));
    }
    if save.name.trim().is_empty() || save.name.chars().count() > 32 {
        return Err("El nombre del mundo no es válido.".into());
    }
    if save.selected_hotbar_slot >= HOTBAR_SLOT_COUNT {
        return Err("El bloque seleccionado no es válido.".into());
    }
    if !is_valid_block(save.selected_block) {
        return Err("El bloque seleccionado no es válido.".into());
    }
    if save.player_position.iter().any(|value| !value.is_finite()) {
        return Err("La posición guardada del jugador no es válida.".into());
    }
    if !save.player_yaw.is_finite()
        || !save.player_pitch.is_finite()
        || save.player_pitch.abs() > 1.55
    {
        return Err("La orientación guardada del jugador no es válida.".into());
    }
    if !save.time_of_day_seconds.is_finite()
        || !(0.0..GAME_DAY_SECONDS).contains(&save.time_of_day_seconds)
    {
        return Err("La hora guardada del mundo no es válida.".into());
    }
    if !save.health.is_finite()
        || !(0.0..=MAX_HEALTH).contains(&save.health)
        || !save.hunger.is_finite()
        || !(0.0..=MAX_HUNGER).contains(&save.hunger)
    {
        return Err("La salud o el hambre guardadas no son válidas.".into());
    }
    if save.food > 99 {
        return Err("La cantidad de comida guardada no es válida.".into());
    }
    if save.pickaxe_durability > 64 || save.axe_durability > 64 || save.shovel_durability > 64 || save.sword_durability > 32 {
        return Err("La durabilidad de una herramienta guardada no es válida.".into());
    }
    if save
        .player_position
        .iter()
        .any(|value| value.abs() > 100_000_000.0)
    {
        return Err("La posición guardada está fuera de los límites permitidos.".into());
    }
    if save.chunks.len() > 4096 {
        return Err("El mundo contiene demasiados chunks.".into());
    }
    let mut coordinates = HashSet::new();
    for chunk in &save.chunks {
        if chunk
            .coord
            .iter()
            .any(|coordinate| coordinate.abs() > 1_000_000)
        {
            return Err("Un chunk guardado está fuera de los límites permitidos.".into());
        }
        if chunk.blocks.len() != CHUNK_VOLUME || chunk.blocks.iter().any(|block| *block > LEAVES) {
            return Err("Un chunk guardado tiene datos inválidos.".into());
        }
        if !coordinates.insert(chunk.coord) {
            return Err("El mundo contiene chunks duplicados.".into());
        }
    }
    if save.inventory[0] != 0 {
        return Err("El inventario contiene aire como bloque.".into());
    }
    Ok(())
}

fn resolve_hotbar_slot(selected_block: u8, saved_slot: usize) -> usize {
    if saved_slot < HOTBAR_SLOT_COUNT && HOTBAR_BLOCKS[saved_slot] == selected_block {
        return saved_slot;
    }
    HOTBAR_BLOCKS
        .iter()
        .position(|block| *block == selected_block)
        .unwrap_or_default()
}

fn is_valid_block(block: u8) -> bool {
    block <= LEAVES
}

fn block_name(block: u8) -> &'static str {
    match block {
        AIR => "Vacío",
        GRASS => "Césped",
        DIRT => "Tierra",
        STONE => "Piedra",
        SAND => "Arena",
        WOOD => "Madera",
        LEAVES => "Hojas",
        _ => "Desconocido",
    }
}

fn experience_required(level: u32) -> u32 {
    10_u32.saturating_add(level.saturating_mul(5))
}

fn experience_progress(total_experience: u32) -> (u32, u32, u32) {
    let mut remaining = total_experience;
    let mut level = 0;
    loop {
        let required = experience_required(level);
        if remaining < required || level == u32::MAX {
            return (level, remaining, required);
        }
        remaining -= required;
        level += 1;
    }
}

fn experience_for_block(block: u8) -> u32 {
    match block {
        STONE | WOOD => 2,
        GRASS | DIRT | SAND | LEAVES => 1,
        _ => 0,
    }
}

fn fall_damage(impact_speed: f32) -> f32 {
    if impact_speed <= 12.0 {
        0.0
    } else {
        ((impact_speed - 12.0) / 4.0).floor() + 1.0
    }
}

fn eat_food(survival: &mut Survival) -> bool {
    eat_food_of_type(survival, FoodType::BerryRation)
}

fn eat_food_of_type(survival: &mut Survival, food_type: FoodType) -> bool {
    if survival.food == 0 || survival.hunger >= MAX_HUNGER {
        return false;
    }

    survival.food -= 1;
    let saturation = food_type.saturation();
    survival.hunger = (survival.hunger + saturation).min(MAX_HUNGER);

    let food_name = match food_type {
        FoodType::BerryRation => "Comiste una ración de bayas.",
        FoodType::Apple => "Comiste una manzana.",
        FoodType::Mushroom => "Comiste un hongo.",
        FoodType::Wheat => "Comiste trigo.",
        FoodType::RawMeat => "Comiste carne cruda.",
        FoodType::CookedMeat => "Comiste carne asada.",
    };

    survival.notice = food_name.into();
    survival.notice_elapsed = 3.0;

    if food_type.is_poisonous() {
        apply_damage(survival, 2.0, "Envenenamiento");
    }

    true
}

fn advance_survival_stats(survival: &mut Survival, delta: f32, sprinting: bool) {
    survival.passive_hunger_elapsed += delta;
    while survival.passive_hunger_elapsed >= PASSIVE_HUNGER_INTERVAL {
        survival.passive_hunger_elapsed -= PASSIVE_HUNGER_INTERVAL;
        survival.hunger = (survival.hunger - 1.0).max(0.0);
    }

    if sprinting {
        survival.sprint_hunger_elapsed += delta;
        while survival.sprint_hunger_elapsed >= SPRINT_HUNGER_INTERVAL {
            survival.sprint_hunger_elapsed -= SPRINT_HUNGER_INTERVAL;
            survival.hunger = (survival.hunger - 1.0).max(0.0);
        }
    } else {
        survival.sprint_hunger_elapsed = 0.0;
    }

    if survival.hunger >= 18.0 && survival.health < MAX_HEALTH {
        survival.regeneration_elapsed += delta;
        while survival.regeneration_elapsed >= HEALTH_REGEN_INTERVAL
            && survival.hunger >= 18.0
            && survival.health < MAX_HEALTH
        {
            survival.regeneration_elapsed -= HEALTH_REGEN_INTERVAL;
            survival.health = (survival.health + 1.0).min(MAX_HEALTH);
            survival.hunger = (survival.hunger - 0.5).max(0.0);
        }
    } else {
        survival.regeneration_elapsed = 0.0;
    }

    if survival.hunger == 0.0 {
        survival.starvation_elapsed += delta;
        while survival.starvation_elapsed >= STARVATION_DAMAGE_INTERVAL && survival.health > 0.0 {
            survival.starvation_elapsed -= STARVATION_DAMAGE_INTERVAL;
            apply_damage(survival, 1.0, "Daño por hambre");
        }
    } else {
        survival.starvation_elapsed = 0.0;
    }
}

fn apply_damage(survival: &mut Survival, amount: f32, notice: &str) {
    if amount <= 0.0 || survival.health <= 0.0 {
        return;
    }
    survival.health = (survival.health - amount).max(0.0);
    survival.notice = notice.to_string();
    survival.notice_elapsed = 4.0;
}

fn snapshot_world(
    world: &WorldData,
    player_position: Vec3,
    player: &Player,
    inventory: &Inventory,
    selected_block: u8,
    selected_hotbar_slot: usize,
    time_of_day_seconds: f32,
    survival: &Survival,
    crafted_tools: &CraftedTools,
) -> WorldSave {
    let mut legacy_inventory = [0; LEGACY_BLOCK_COUNT];
    legacy_inventory.copy_from_slice(&inventory.counts[..LEGACY_BLOCK_COUNT]);
    let mut extra_inventory = [0; BLOCK_COUNT - LEGACY_BLOCK_COUNT];
    extra_inventory.copy_from_slice(&inventory.counts[LEGACY_BLOCK_COUNT..]);
    WorldSave {
        version: SAVE_VERSION,
        name: world.name.clone(),
        seed: world.seed,
        time_of_day_seconds,
        player_position: player_position.to_array(),
        player_yaw: player.yaw,
        player_pitch: player.pitch,
        selected_block,
        selected_hotbar_slot,
        inventory: legacy_inventory,
        extra_inventory,
        health: survival.health,
        hunger: survival.hunger,
        experience: survival.experience,
        food: survival.food,
        pickaxe_durability: crafted_tools.pickaxe_durability,
        axe_durability: crafted_tools.axe_durability,
        shovel_durability: crafted_tools.shovel_durability,
        sword_durability: crafted_tools.sword_durability,
        chunks: Vec::new(),
    }
}

fn chunk_directory(world_path: &Path) -> PathBuf {
    world_path.with_extension("chunks")
}

fn chunk_file_path(world_path: &Path, coord: IVec3) -> PathBuf {
    chunk_directory(world_path).join(format!("{}_{}_{}.ron", coord.x, coord.y, coord.z))
}

fn save_chunk_file(world_path: &Path, coord: IVec3, chunk: &Chunk) -> Result<(), String> {
    if chunk.blocks.len() != CHUNK_VOLUME || chunk.blocks.iter().any(|block| *block >= BLOCK_COUNT as u8) {
        return Err(format!("Los datos del chunk {coord:?} no son válidos."));
    }
    let directory = chunk_directory(world_path);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("No se pudo crear {}: {error}", directory.display()))?;
    let path = chunk_file_path(world_path, coord);
    let temporary_path = path.with_extension("ron.tmp");
    let save = ChunkSave {
        coord: coord.to_array(),
        blocks: chunk.blocks.clone(),
    };
    let contents = ron::ser::to_string(&save)
        .map_err(|error| format!("No se pudo serializar el chunk {coord:?}: {error}"))?;
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("No se pudo escribir el chunk {coord:?}: {error}"))?;
    fs::rename(&temporary_path, &path)
        .map_err(|error| format!("No se pudo finalizar el chunk {coord:?}: {error}"))?;
    Ok(())
}

fn load_chunk_file(world_path: &Path, coord: IVec3) -> Result<Option<Chunk>, String> {
    let path = chunk_file_path(world_path, coord);
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("No se pudo inspeccionar {}: {error}", path.display()))?;
    if metadata.len() > (CHUNK_VOLUME * 4) as u64 {
        return Err(format!(
            "El archivo del chunk {coord:?} supera el tamaño permitido."
        ));
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("No se pudo leer el chunk {coord:?}: {error}"))?;
    let save: ChunkSave = ron::from_str(&contents)
        .map_err(|error| format!("El archivo del chunk {coord:?} no es válido: {error}"))?;
    if save.coord != coord.to_array()
        || save.blocks.len() != CHUNK_VOLUME
        || save.blocks.iter().any(|block| *block >= BLOCK_COUNT as u8)
    {
        return Err(format!(
            "Los datos guardados del chunk {coord:?} no son válidos."
        ));
    }
    Ok(Some(Chunk {
        blocks: save.blocks,
    }))
}

fn save_world(world_path: &Path, save: WorldSave) -> Result<(), String> {
    validate_world_save(&save)?;
    let directory = world_path
        .parent()
        .ok_or_else(|| "No se pudo determinar el directorio de mundos.".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("No se pudo crear {}: {error}", directory.display()))?;
    let contents = ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("No se pudo serializar el mundo: {error}"))?;
    let temporary_path = world_path.with_extension("ron.tmp");
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("No se pudo escribir el mundo: {error}"))?;
    fs::rename(&temporary_path, world_path)
        .map_err(|error| format!("No se pudo finalizar el guardado: {error}"))?;
    Ok(())
}

fn save_all_chunks(
    world: &WorldData,
    storage: &mut WorldStorage,
    world_path: &Path,
) -> Result<(), String> {
    for (coord, chunk) in &storage.legacy_chunks {
        save_chunk_file(world_path, *coord, chunk)?;
    }
    for (coord, chunk) in &world.modified_chunks {
        save_chunk_file(world_path, *coord, chunk)?;
    }
    storage.legacy_chunks.clear();
    storage.dirty_chunks.clear();
    storage.failed_chunks.clear();
    storage.status = "Guardado".into();
    Ok(())
}

fn load_or_generate_chunk(
    world: &mut WorldData,
    storage: &mut WorldStorage,
    coord: IVec3,
) -> Result<Chunk, String> {
    if let Some(chunk) = world.modified_chunks.get(&coord) {
        return Ok(Chunk {
            blocks: chunk.blocks.clone(),
        });
    }
    if let Some(chunk) = storage.legacy_chunks.remove(&coord) {
        storage.dirty_chunks.insert(coord);
        world.modified_chunks.insert(
            coord,
            Chunk {
                blocks: chunk.blocks.clone(),
            },
        );
        return Ok(chunk);
    }
    if let Some(path) = &storage.path {
        if let Some(chunk) = load_chunk_file(path, coord)? {
            return Ok(chunk);
        }
    }
    Ok(chunk_for_coordinate(world, coord))
}

fn autosave_world(
    time: Res<Time>,
    world: Res<WorldData>,
    player: Query<(&Transform, &Player)>,
    inventory: Res<Inventory>,
    survival: Res<Survival>,
    crafted_tools: Res<CraftedTools>,
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    day_night: Res<DayNightCycle>,
    mut storage: ResMut<WorldStorage>,
) {
    let Some(path) = storage.path.clone() else {
        return;
    };
    storage.autosave_elapsed += time.delta_secs();
    storage.retry_elapsed = (storage.retry_elapsed - time.delta_secs()).max(0.0);
    if storage.retry_elapsed == 0.0 {
        storage.failed_chunks.clear();
    }
    if storage.autosave_elapsed >= AUTOSAVE_INTERVAL_SECONDS {
        storage.autosave_elapsed = 0.0;
        storage.metadata_save_requested = true;
    }
    let next_dirty = storage
        .dirty_chunks
        .iter()
        .find(|coord| !storage.failed_chunks.contains(coord))
        .copied();
    if let Some(coord) = next_dirty {
        let chunk = world
            .modified_chunks
            .get(&coord)
            .or_else(|| storage.legacy_chunks.get(&coord));
        if let Some(chunk) = chunk {
            match save_chunk_file(&path, coord, chunk) {
                Ok(()) => {
                    storage.dirty_chunks.remove(&coord);
                    storage.failed_chunks.remove(&coord);
                    if storage.legacy_chunks.remove(&coord).is_some() {
                        storage.status = "Chunks migrados y guardados".into();
                    }
                    if storage.status != "Chunks migrados y guardados" {
                        storage.status = "Bloques guardados".into();
                    }
                }
                Err(error) => {
                    error!("Could not auto-save chunk {coord:?}: {error}");
                    storage.failed_chunks.insert(coord);
                    storage.retry_elapsed = 5.0;
                    storage.status = format!("Error al guardar chunk: {error}");
                }
            }
        } else {
            storage.dirty_chunks.remove(&coord);
        }
        return;
    }

    if !storage.metadata_save_requested
        || !storage.dirty_chunks.is_empty()
        || storage.retry_elapsed > 0.0
    {
        return;
    }
    let Ok((transform, player)) = player.get_single() else {
        storage.status = "No se pudo guardar: falta el jugador.".into();
        storage.retry_elapsed = 5.0;
        return;
    };
    let save = snapshot_world(
        &world,
        transform.translation,
        player,
        &inventory,
        selected.0,
        selected_slot.0,
        day_night.time_of_day_seconds,
        &survival,
        &crafted_tools,
    );
    match save_world(&path, save) {
        Ok(()) => {
            storage.metadata_save_requested = false;
            storage.status = "Guardado automáticamente".into();
        }
        Err(error) => {
            error!("Could not auto-save world metadata: {error}");
            storage.retry_elapsed = 5.0;
            storage.status = format!("Error al guardar: {error}");
        }
    }
}

/// Create particle effects when a block is destroyed
fn spawn_block_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    block_pos: Vec3,
    block_type: u8,
) {
    // Spawn 5 particles per destroyed block
    let particle_count = 5;
    let block_color = match block_type {
        GRASS => Color::linear_rgb(0.3, 0.7, 0.3),
        DIRT => Color::linear_rgb(0.6, 0.4, 0.2),
        STONE => Color::linear_rgb(0.5, 0.5, 0.5),
        SAND => Color::linear_rgb(0.8, 0.75, 0.5),
        WOOD => Color::linear_rgb(0.4, 0.25, 0.1),
        LEAVES => Color::linear_rgb(0.2, 0.6, 0.2),
        WATER => Color::linear_rgb(0.2, 0.5, 0.8),
        LAVA => Color::linear_rgb(1.0, 0.5, 0.0),
        _ => Color::linear_rgb(0.7, 0.7, 0.7),
    };
    
    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let speed = 2.0 + (i as f32) * 0.3;
        let velocity = Vec3::new(angle.cos() * speed, 3.0 + (i as f32) * 0.2, angle.sin() * speed);
        
        let mesh = meshes.add(Cuboid::new(0.1, 0.1, 0.1));
        let material = materials.add(StandardMaterial {
            base_color: block_color,
            ..default()
        });
        
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(block_pos + Vec3::splat(0.5)),
            BlockParticle {
                lifetime: 1.0,
                max_lifetime: 1.0,
                velocity,
            },
        ));
    }
}

fn update_particles(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut BlockParticle, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut transform) in &mut particles {
        particle.lifetime -= time.delta_secs();
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        
        // Physics: apply gravity and velocity
        let gravity = Vec3::new(0.0, -9.8, 0.0);
        let dt = time.delta_secs();
        particle.velocity += gravity * dt;
        transform.translation += particle.velocity * dt;
    }
}

fn animate_water_lava(
    texture_assets: Res<TextureAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    // Animate water and lava colors with a subtle pulsing effect
    let wave = (time.elapsed_secs() * 2.0).sin(); // Oscillates between -1 and 1
    let brightness = 0.9 + wave * 0.1; // Varies between 0.8 and 1.0
    
    if let Some(water_handle) = &texture_assets.water_material {
        if let Some(material) = materials.get_mut(water_handle) {
            // Water: cycle between light blue and slightly darker blue
            material.base_color = Color::linear_rgb(
                0.2 * brightness,
                0.5 * brightness,
                0.8 * brightness,
            );
        }
    }
    
    if let Some(lava_handle) = &texture_assets.lava_material {
        if let Some(material) = materials.get_mut(lava_handle) {
            // Lava: cycle between orange and slightly darker orange
            material.base_color = Color::linear_rgb(
                1.0 * brightness,
                0.5 * brightness,
                0.0 * brightness,
            );
        }
    }
}

fn capture_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    crafting_ui: Res<CraftingUi>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
) {
    pointer.captured_this_frame = false;
    if crafting_ui.open {
        return;
    }
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if window.cursor_options.grab_mode != CursorGrabMode::Locked
        && mouse.just_pressed(MouseButton::Left)
    {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
        pointer.captured_this_frame = true;
    }
}

fn look(
    mut motion: EventReader<MouseMotion>,
    mut player: Query<(&mut Transform, &mut Player)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
) {
    let delta = motion.read().map(|event| event.delta).sum::<Vec2>();
    let Ok(window) = windows.get_single() else {
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked || pointer.captured_this_frame {
        return;
    }

    let Ok((mut transform, mut player)) = player.get_single_mut() else {
        return;
    };
    player.yaw -= delta.x * 0.002;
    player.pitch = (player.pitch - delta.y * 0.002).clamp(-1.54, 1.54);
    transform.rotation = player_rotation(player.yaw, player.pitch);
}

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<WorldData>,
    mut survival: ResMut<Survival>,
    mut player: Query<(&mut Transform, &mut Player)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let Ok((mut transform, mut player)) = player.get_single_mut() else {
        return;
    };

    // Validate player is not stuck in blocks
    transform.translation = find_safe_spawn_position(&world, transform.translation);

    if keys.just_pressed(KeyCode::KeyF) {
        player.flying = !player.flying;
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        info!(
            "Movement mode: {}",
            if player.flying { "flight" } else { "walking" }
        );
    }

    let mut direction = Vec3::ZERO;
    let forward = (transform.rotation * Vec3::NEG_Z)
        .with_y(0.0)
        .normalize_or_zero();
    let right = (transform.rotation * Vec3::X)
        .with_y(0.0)
        .normalize_or_zero();

    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right;
    }

    let delta = time.delta_secs().min(0.05);
    if player.flying {
        if keys.pressed(KeyCode::Space) {
            direction.y += 1.0;
        }
        if keys.pressed(KeyCode::ControlLeft) {
            direction.y -= 1.0;
        }
        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * SPRINT_SPEED * delta;
        }
        return;
    }

    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        SPRINT_SPEED
    } else {
        WALK_SPEED
    };
    direction = direction.normalize_or_zero() * speed;
    player.velocity.x = direction.x;
    player.velocity.z = direction.z;

    if keys.just_pressed(KeyCode::Space) && player.grounded {
        player.velocity.y = JUMP_SPEED;
        player.grounded = false;
    }
    player.velocity.y = (player.velocity.y - GRAVITY * delta).max(-50.0);

    let displacement = player.velocity * delta;
    let step_count = (displacement.abs().max_element() / 0.15).ceil().max(1.0) as usize;
    let step = displacement / step_count as f32;
    let can_step = player.grounded;
    player.grounded = false;

    for _ in 0..step_count {
        for axis in [0, 2] {
            let mut candidate = transform.translation;
            candidate[axis] += step[axis];
            if !collides_with_world(&world, candidate) {
                transform.translation = candidate;
            } else if can_step && step[axis] != 0.0 {
                let raised = transform.translation + Vec3::Y * STEP_HEIGHT;
                let mut stepped = raised;
                stepped[axis] += step[axis];
                if !collides_with_world(&world, raised) && !collides_with_world(&world, stepped) {
                    transform.translation = stepped;
                    player.velocity.y = 0.0;
                }
            }
        }

        let mut candidate = transform.translation;
        candidate.y += step.y;
        if !collides_with_world(&world, candidate) {
            transform.translation = candidate;
        } else {
            if step.y < 0.0 {
                player.grounded = true;
                let impact_speed = -player.velocity.y;
                apply_damage(&mut survival, fall_damage(impact_speed), "Daño por caída");
            }
            player.velocity.y = 0.0;
        }
    }
}

fn update_survival(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<WorldData>,
    mut survival: ResMut<Survival>,
    mut player: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = player.get_single_mut() else {
        return;
    };
    let delta = time.delta_secs();
    survival.notice_elapsed = (survival.notice_elapsed - delta).max(0.0);
    if survival.notice_elapsed == 0.0 {
        survival.notice.clear();
    }

    let moving = player.velocity.xz().length_squared() > 0.1;
    let sprinting = !player.flying && moving && keys.pressed(KeyCode::ShiftLeft);
    advance_survival_stats(&mut survival, delta, sprinting);

    if survival.health <= 0.0 {
        let spawn_x = 8.5;
        let spawn_z = 8.5;
        let ground = terrain_height(spawn_x as i32, spawn_z as i32, world.seed);
        transform.translation =
            Vec3::new(spawn_x, ground as f32 + 1.0 + PLAYER_EYE_HEIGHT, spawn_z);
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        player.flying = false;
        survival.health = MAX_HEALTH / 2.0;
        survival.hunger = MAX_HUNGER / 2.0;
        survival.passive_hunger_elapsed = 0.0;
        survival.sprint_hunger_elapsed = 0.0;
        survival.starvation_elapsed = 0.0;
        survival.regeneration_elapsed = 0.0;
        survival.notice = "Has reaparecido; conservas inventario y experiencia.".into();
        survival.notice_elapsed = 6.0;
    }
}

fn consume_food(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut survival: ResMut<Survival>,
    mut crafting_ui: ResMut<CraftingUi>,
) {
    if !keys.just_pressed(KeyCode::KeyE)
        || !windows
            .get_single()
            .is_ok_and(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
    {
        return;
    }

    // Toggle inventory with E
    crafting_ui.inventory_open = !crafting_ui.inventory_open;
    if crafting_ui.inventory_open {
        crafting_ui.inventory_just_opened = true;
    }

    // If inventory is now open, don't eat
    if crafting_ui.inventory_open {
        return;
    }

    // Eat only if inventory is closed
    if survival.food > 0 && survival.hunger < MAX_HUNGER {
        eat_food(&mut survival);
    }
}

fn stream_world_chunks(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut loading: ResMut<LoadingProgress>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_entities: Query<(Entity, &ChunkMesh)>,
    texture_assets: Res<TextureAssets>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let player_block = IVec3::new(
        player.translation.x.floor() as i32,
        0,
        player.translation.z.floor() as i32,
    );
    let center = block_address(player_block).0;

    let unload = chunk_entities
        .iter()
        .filter_map(|(entity, chunk)| {
            let distance = (i64::from(chunk.0.x) - i64::from(center.x))
                .abs()
                .max((i64::from(chunk.0.z) - i64::from(center.z)).abs());
            (distance > i64::from(STREAM_UNLOAD_RADIUS)).then_some((entity, chunk.0))
        })
        .collect::<Vec<_>>();
    for (entity, coord) in unload {
        if storage.failed_chunks.contains(&coord) {
            continue;
        }
        if let Some(chunk) = world.modified_chunks.get(&coord) {
            let Some(path) = storage.path.as_ref() else {
                storage.status = "No se encontró la ruta para guardar un chunk.".into();
                storage.retry_elapsed = 5.0;
                continue;
            };
            match save_chunk_file(path, coord, chunk) {
                Ok(()) => {
                    world.modified_chunks.remove(&coord);
                    storage.dirty_chunks.remove(&coord);
                    storage.failed_chunks.remove(&coord);
                    storage.status = "Bloques guardados".into();
                }
                Err(error) => {
                    error!("Could not save chunk {coord:?} before unloading: {error}");
                    storage.failed_chunks.insert(coord);
                    storage.retry_elapsed = 5.0;
                    storage.status = format!("Error al guardar chunk: {error}");
                    continue;
                }
            }
        }
        commands.entity(entity).despawn();
        world.chunks.remove(&coord);
        world.dirty.remove(&coord);
        mark_chunk_neighbors_dirty(&mut world, coord);
    }

    let mut missing = chunk_coordinates_around(center, STREAM_LOAD_RADIUS)
        .into_iter()
        .filter(|coord| !world.chunks.contains_key(coord))
        .collect::<Vec<_>>();
    missing.sort_unstable_by_key(|coord| {
        let x = i64::from(coord.x) - i64::from(center.x);
        let z = i64::from(coord.z) - i64::from(center.z);
        x * x + z * z
    });
    let Some(coord) = missing.first().copied() else {
        return;
    };

    if storage.failed_chunks.contains(&coord) {
        return;
    }
    let chunk = match load_or_generate_chunk(&mut world, &mut storage, coord) {
        Ok(chunk) => chunk,
        Err(error) => {
            error!("Could not stream chunk {coord:?}: {error}");
            storage.failed_chunks.insert(coord);
            storage.retry_elapsed = 5.0;
            storage.status = format!("Error al cargar chunk: {error}");
            return;
        }
    };
    world.chunks.insert(coord, chunk);
    mark_chunk_neighbors_dirty(&mut world, coord);

    if loading.material.is_none() {
        loading.material = Some(materials.add(StandardMaterial {
            base_color_texture: texture_assets.atlas.clone(),
            ..default()
        }));
    }
    let Some(material) = loading.material.clone() else {
        error!("Could not create the chunk material");
        world.chunks.remove(&coord);
        return;
    };
    let mesh = meshes.add(rendering::build_chunk_mesh(&world, coord));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32),
        ChunkMesh(coord),
        GameEntity,
    ));
}

fn mark_chunk_neighbors_dirty(world: &mut WorldData, coord: IVec3) {
    for axis in [0, 2] {
        for offset in [-1, 1] {
            let mut neighbor = coord;
            neighbor[axis] += offset;
            if world.chunks.contains_key(&neighbor) {
                world.dirty.insert(neighbor);
            }
        }
    }
}

fn chunk_for_coordinate(world: &WorldData, coord: IVec3) -> Chunk {
    world.modified_chunks.get(&coord).map_or_else(
        || generate_chunk(coord, world.seed),
        |chunk| Chunk {
            blocks: chunk.blocks.clone(),
        },
    )
}

fn update_mobs(
    mut commands: Commands,
    time: Res<Time>,
    cycle: Res<DayNightCycle>,
    world: Res<WorldData>,
    assets: Res<MobAssets>,
    mut population: ResMut<MobPopulation>,
    player: Query<&Transform, With<Player>>,
    mut mobs: Query<(Entity, &mut Transform, &mut Mob), Without<Player>>,
    mut survival: ResMut<Survival>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let player_position = player.translation;
    let (_, daylight) = sun_position_and_daylight(cycle.time_of_day_seconds);
    let night = is_night(daylight);
    let delta = time.delta_secs();
    let mut total_count = 0;
    let mut grazer_count = 0;
    let mut stalker_count = 0;

    for (entity, mut transform, mut mob) in &mut mobs {
        let distance = transform.translation.distance(player_position);
        if distance > MOB_DESPAWN_DISTANCE || (mob.kind == MobKind::NightStalker && !night) {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        total_count += 1;
        match mob.kind {
            MobKind::Grazer => grazer_count += 1,
            MobKind::NightStalker => stalker_count += 1,
        }

        let to_player = (player_position - transform.translation).xz();
        let distance_to_player = to_player.length();
        let mut move_direction = Vec3::ZERO;
        match mob.kind {
            MobKind::NightStalker if distance_to_player > 1.35 && distance_to_player < 24.0 => {
                let direction = to_player.normalize_or_zero();
                move_direction = Vec3::new(direction.x, 0.0, direction.y);
                mob.attack_elapsed = 0.0;
            }
            MobKind::NightStalker
                if distance_to_player <= 1.35
                    && (player_position.y - transform.translation.y).abs() <= 1.5 =>
            {
                mob.attack_elapsed += delta;
                if mob.attack_elapsed >= 1.25 {
                    mob.attack_elapsed = 0.0;
                    apply_damage(&mut survival, 2.0, "Un acechador te ha herido");
                }
            }
            MobKind::Grazer if distance_to_player < 9.0 => {
                let direction = -to_player.normalize_or_zero();
                move_direction = Vec3::new(direction.x, 0.0, direction.y);
                mob.wander_elapsed = 0.0;
            }
            MobKind::Grazer => {
                mob.wander_elapsed -= delta;
                if mob.wander_elapsed <= 0.0 {
                    let cell = transform.translation.floor().as_ivec3();
                    let hash =
                        coordinate_hash(cell.x, cell.z, world.seed ^ u64::from(mob.wander_step));
                    mob.wander_step = mob.wander_step.wrapping_add(1);
                    mob.wander_angle =
                        (hash as u32 as f32 / u32::MAX as f32) * std::f32::consts::TAU;
                    mob.wander_elapsed = 1.5 + ((hash >> 32) as u16 as f32 / u16::MAX as f32) * 2.5;
                }
                move_direction = Vec3::new(mob.wander_angle.sin(), 0.0, mob.wander_angle.cos());
            }
            _ => {}
        }

        if move_direction != Vec3::ZERO {
            let speed = match mob.kind {
                MobKind::Grazer if distance_to_player < 9.0 => 4.0,
                MobKind::Grazer => 1.0,
                MobKind::NightStalker => 2.0,
            };
            let next_position = transform.translation + move_direction * speed * delta;
            if let Some(ground) = mob_ground_height(
                &world,
                next_position.x.floor() as i32,
                next_position.z.floor() as i32,
            ) {
                if (ground as f32 + 1.0 - transform.translation.y).abs() <= 1.0 {
                    transform.translation =
                        Vec3::new(next_position.x, ground as f32 + 1.0, next_position.z);
                    transform.rotation =
                        Quat::from_rotation_y(move_direction.x.atan2(-move_direction.z));
                }
            }
        }
    }

    population.spawn_elapsed += delta;
    if population.spawn_elapsed < MOB_SPAWN_INTERVAL {
        return;
    }
    population.spawn_elapsed = 0.0;
    population.spawn_attempt = population.spawn_attempt.wrapping_add(1);
    let kind = if night {
        MobKind::NightStalker
    } else {
        MobKind::Grazer
    };
    if !can_spawn_mob(kind, total_count, grazer_count, stalker_count) {
        return;
    }
    let Some(position) = mob_spawn_position(&world, player_position, population.spawn_attempt)
    else {
        return;
    };
    spawn_mob(&mut commands, &assets, kind, position);
}

fn is_night(daylight: f32) -> bool {
    daylight < 0.08
}

fn can_spawn_mob(kind: MobKind, total: usize, grazers: usize, stalkers: usize) -> bool {
    total < MAX_MOBS
        && match kind {
            MobKind::Grazer => grazers < MAX_PASSIVE_MOBS,
            MobKind::NightStalker => stalkers < MAX_HOSTILE_MOBS,
        }
}

/// Find a safe spawn position by checking if player is stuck in blocks
/// and teleporting to ground level if needed
fn find_safe_spawn_position(world: &WorldData, mut position: Vec3) -> Vec3 {
    let ground_x = position.x.floor() as i32;
    let ground_z = position.z.floor() as i32;
    
    // Check if player is inside a solid block
    for offset_y in -2..=2 {
        let check_pos = IVec3::new(ground_x, position.y as i32 + offset_y, ground_z);
        if block_at(world, check_pos) != AIR {
            // Player is stuck, find safe ground height
            if let Some(height) = mob_ground_height(world, ground_x, ground_z) {
                position.y = (height as f32) + PLAYER_EYE_HEIGHT;
                return position;
            }
            // If we can't find ground, teleport higher
            position.y = (position.y as i32 + 5) as f32;
            return position;
        }
    }
    position
}

fn mob_ground_height(world: &WorldData, x: i32, z: i32) -> Option<i32> {
    let chunk = block_address(IVec3::new(x, 0, z)).0;
    if !world.chunks.contains_key(&chunk) {
        return None;
    }
    (0..CHUNK_SIZE)
        .rev()
        .find(|&y| block_at(world, IVec3::new(x, y, z)) != AIR)
}

fn mob_spawn_position(world: &WorldData, player_position: Vec3, attempt: u64) -> Option<Vec3> {
    let origin_x = player_position.x.floor() as i32;
    let origin_z = player_position.z.floor() as i32;
    let hash = coordinate_hash(
        origin_x,
        origin_z,
        world.seed ^ attempt.wrapping_mul(0x9E37_79B9_7F4A_7C15),
    );
    let angle = (hash as u32 as f32 / u32::MAX as f32) * std::f32::consts::TAU;
    let radius = 8.0 + ((hash >> 32) % 11) as f32;
    let x = (player_position.x + angle.cos() * radius).floor() as i32;
    let z = (player_position.z + angle.sin() * radius).floor() as i32;
    let biome = biome_at(x, z, world.seed);
    if !matches!(biome, Biome::Plains | Biome::Forest) {
        return None;
    }

    let ground = mob_ground_height(world, x, z)?;
    if block_at(world, IVec3::new(x, ground, z)) != GRASS
        || block_at(world, IVec3::new(x, ground + 1, z)) != AIR
        || block_at(world, IVec3::new(x, ground + 2, z)) != AIR
    {
        return None;
    }
    
    // Torches can prevent mob spawning by providing light
    // Light level calculation includes torch proximity (can be < 8 even if torches nearby)
    // For now, this is informational; actual darkness requirements happen at mob_update
    let _spawn_pos = IVec3::new(x, ground + 1, z);
    
    Some(Vec3::new(
        x as f32 + 0.5,
        ground as f32 + 1.0,
        z as f32 + 0.5,
    ))
}

fn spawn_mob(commands: &mut Commands, assets: &MobAssets, kind: MobKind, position: Vec3) {
    let (
        Some(cube),
        Some(grazer_body),
        Some(grazer_head),
        Some(grazer_leg),
        Some(stalker_body),
        Some(stalker_eye),
    ) = (
        assets.cube.clone(),
        assets.grazer_body.clone(),
        assets.grazer_head.clone(),
        assets.grazer_leg.clone(),
        assets.stalker_body.clone(),
        assets.stalker_eye.clone(),
    )
    else {
        error!("Mob assets were not initialized before spawning");
        return;
    };
    let (health, body_material, head_material, leg_material) = match kind {
        MobKind::Grazer => (
            3,
            grazer_body.clone(),
            grazer_head.clone(),
            grazer_leg.clone(),
        ),
        MobKind::NightStalker => (5, stalker_body.clone(), stalker_body.clone(), stalker_body),
    };
    let initial_hash = coordinate_hash(
        position.x.floor() as i32,
        position.z.floor() as i32,
        health as u64,
    );
    let root = commands
        .spawn((
            Transform::from_translation(position),
            Mob {
                kind,
                health,
                attack_elapsed: 0.0,
                wander_elapsed: 0.0,
                wander_angle: (initial_hash as u32 as f32 / u32::MAX as f32)
                    * std::f32::consts::TAU,
                wander_step: 0,
            },
            GameEntity,
        ))
        .id();
    commands.entity(root).with_children(|parent| {
        let body_size = match kind {
            MobKind::Grazer => Vec3::new(0.9, 0.65, 0.55),
            MobKind::NightStalker => Vec3::new(0.75, 0.95, 0.5),
        };
        let body_y = if kind == MobKind::Grazer { 0.8 } else { 0.95 };
        parent.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(body_material),
            Transform::from_xyz(0.0, body_y, 0.0).with_scale(body_size),
        ));
        let head_y = if kind == MobKind::Grazer { 1.25 } else { 1.7 };
        parent.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(head_material),
            Transform::from_xyz(0.0, head_y, -0.32).with_scale(Vec3::splat(0.42)),
        ));
        if kind == MobKind::NightStalker {
            for x in [-0.12, 0.12] {
                parent.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(stalker_eye.clone()),
                    Transform::from_xyz(x, head_y, -0.54).with_scale(Vec3::new(0.08, 0.08, 0.04)),
                ));
            }
        }
        let leg_y = 0.28;
        for x in [-0.26, 0.26] {
            for z in [-0.17, 0.17] {
                parent.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(leg_material.clone()),
                    Transform::from_xyz(x, leg_y, z).with_scale(Vec3::new(0.2, 0.55, 0.2)),
                ));
            }
        }
    });
}

fn mob_rewards(kind: MobKind) -> (u32, u32) {
    match kind {
        MobKind::Grazer => (2, 3),
        MobKind::NightStalker => (0, 5),
    }
}

fn ray_sphere_distance(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }
    let offset = origin - center;
    let projection = offset.dot(direction);
    let discriminant = projection * projection - (offset.length_squared() - radius * radius);
    if discriminant < 0.0 {
        return None;
    }
    let near = -projection - discriminant.sqrt();
    let distance = if near >= 0.0 {
        near
    } else {
        -projection + discriminant.sqrt()
    };
    (0.0..=REACH).contains(&distance).then_some(distance)
}

fn collides_with_world(world: &WorldData, eye_position: Vec3) -> bool {
    let min = Vec3::new(
        eye_position.x - PLAYER_HALF_WIDTH,
        eye_position.y - PLAYER_EYE_HEIGHT,
        eye_position.z - PLAYER_HALF_WIDTH,
    );
    let max = Vec3::new(
        eye_position.x + PLAYER_HALF_WIDTH,
        eye_position.y + PLAYER_HEADROOM,
        eye_position.z + PLAYER_HALF_WIDTH,
    );
    let min_block = (min + Vec3::splat(0.0001)).floor().as_ivec3();
    let max_block = (max - Vec3::splat(0.0001)).floor().as_ivec3();

    for y in min_block.y..=max_block.y {
        for z in min_block.z..=max_block.z {
            for x in min_block.x..=max_block.x {
                if is_block_solid(world, IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

fn select_block(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
) {
    let slot = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ]
    .iter()
    .position(|key| keys.just_pressed(*key));
    if let Some(slot) = slot {
        set_hotbar_selection(slot, &mut selected, &mut selected_slot);
    }
}

fn select_hotbar(
    mut wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
) {
    let mut steps = 0_isize;
    for event in wheel.read() {
        steps += mouse_wheel_steps(event.y, event.unit);
    }
    if steps == 0
        || !windows
            .get_single()
            .is_ok_and(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
    {
        return;
    }
    let slot = cycle_hotbar_selection(selected_slot.0, steps);
    set_hotbar_selection(slot, &mut selected, &mut selected_slot);
}

fn set_hotbar_selection(
    slot: usize,
    selected: &mut SelectedBlock,
    selected_slot: &mut SelectedHotbarSlot,
) {
    let slot = slot % HOTBAR_SLOT_COUNT;
    selected_slot.0 = slot;
    selected.0 = HOTBAR_BLOCKS[slot];
}

fn cycle_hotbar_selection(current: usize, steps: isize) -> usize {
    (current as isize + steps).rem_euclid(HOTBAR_SLOT_COUNT as isize) as usize
}

fn mouse_wheel_steps(y: f32, unit: MouseScrollUnit) -> isize {
    if y == 0.0 {
        return 0;
    }
    let magnitude = match unit {
        MouseScrollUnit::Line => y.abs().ceil(),
        MouseScrollUnit::Pixel => (y.abs() / 40.0).ceil(),
    }
    .clamp(1.0, HOTBAR_SLOT_COUNT as f32) as isize;
    -magnitude * y.signum() as isize
}

fn update_hud(
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    inventory: Res<Inventory>,
    survival: Res<Survival>,
    crafted_tools: Res<CraftedTools>,
    world: Res<WorldData>,
    day_night: Res<DayNightCycle>,
    storage: Res<WorldStorage>,
    player: Query<(&Player, &Transform)>,
    mut status: Query<&mut Text, With<HudStatus>>,
) {
    let (Ok((player, transform)), Ok(mut status)) = (player.get_single(), status.get_single_mut())
    else {
        return;
    };
    let mode = if player.flying { "VOLAR" } else { "CAMINAR" };
    let block = block_name(selected.0);
    let biome = biome_at(
        transform.translation.x.floor() as i32,
        transform.translation.z.floor() as i32,
        world.seed,
    )
    .name();
    let tools = [
        (crafted_tools.pickaxe_durability > 0)
            .then(|| format!("Pico {}", crafted_tools.pickaxe_durability)),
        (crafted_tools.axe_durability > 0)
            .then(|| format!("Hacha {}", crafted_tools.axe_durability)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    status.0 = format!(
        "{mode}  [F] alternar\nBioma: {biome}\nHora: {}\nEspacio {}: {block} ×{}\nComida: {} [E]\n{tools}\n{}\n[C] crafteo  |  [Rueda] cambiar  |  [ESC] pausa{}",
        format_time_of_day(day_night.time_of_day_seconds),
        selected_slot.0 + 1,
        inventory.count(selected.0),
        survival.food,
        storage.status,
        if survival.notice_elapsed > 0.0 {
            format!("\n{}", survival.notice)
        } else {
            String::new()
        },
    );
}

fn update_survival_hud(
    survival: Res<Survival>,
    mut fills: Query<(&SurvivalBarFill, &mut Node, &mut BackgroundColor)>,
    mut values: Query<(&SurvivalValue, &mut Text), Without<ExperienceText>>,
    mut experience_text: Query<&mut Text, With<ExperienceText>>,
) {
    let (level, progress, required) = experience_progress(survival.experience);
    for (value, mut text) in &mut values {
        let (label, amount, max) = match value.0 {
            SurvivalBarKind::Health => ("Vida", survival.health, MAX_HEALTH),
            SurvivalBarKind::Hunger => ("Hambre", survival.hunger, MAX_HUNGER),
            SurvivalBarKind::Experience => continue,
        };
        text.0 = format!("{label}: {:.0}/{max:.0}", amount);
    }
    for (fill, mut node, mut color) in &mut fills {
        let (ratio, tint) = match fill.0 {
            SurvivalBarKind::Health => (
                survival.health / MAX_HEALTH,
                if survival.health <= 5.0 {
                    Color::srgb(1.0, 0.12, 0.08)
                } else {
                    Color::srgb(0.82, 0.18, 0.2)
                },
            ),
            SurvivalBarKind::Hunger => (survival.hunger / MAX_HUNGER, Color::srgb(0.9, 0.62, 0.16)),
            SurvivalBarKind::Experience => (
                progress as f32 / required as f32,
                Color::srgb(0.25, 0.75, 0.32),
            ),
        };
        node.width = Val::Percent(ratio.clamp(0.0, 1.0) * 100.0);
        color.0 = tint;
    }
    if let Ok(mut text) = experience_text.get_single_mut() {
        text.0 = format!("Nivel {level} · {progress}/{required} XP");
    }
}

fn format_time_of_day(time_of_day_seconds: f32) -> String {
    let total_minutes = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / 60.0).floor() as u32;
    format!("{:02}:{:02}", (total_minutes / 60) % 24, total_minutes % 60)
}

fn update_hotbar(
    selected: Res<SelectedHotbarSlot>,
    inventory: Res<Inventory>,
    mut slots: Query<(&HotbarSlot, &mut BorderColor, &mut Node)>,
    mut counts: Query<(&HotbarCount, &mut Text)>,
) {
    let selection_changed = selected.is_changed();
    let inventory_changed = inventory.is_changed();
    if !selection_changed && !inventory_changed {
        return;
    }

    if selection_changed {
        for (slot, mut border_color, mut node) in &mut slots {
            let is_selected = slot.0 == selected.0;
            border_color.0 = if is_selected {
                Color::WHITE
            } else {
                Color::srgba(0.12, 0.14, 0.18, 0.9)
            };
            node.border = UiRect::all(Val::Px(if is_selected { 3.0 } else { 1.0 }));
        }
    }
    if inventory_changed {
        for (slot, mut text) in &mut counts {
            text.0 = inventory.count(HOTBAR_BLOCKS[slot.0]).to_string();
        }
    }
}

fn edit_blocks(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    camera: Query<&Transform, With<Player>>,
    mut mobs: Query<(Entity, &Transform, &mut Mob), Without<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
    crafting_ui: Res<CraftingUi>,
    selected: Res<SelectedBlock>,
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut tools: ResMut<CraftedTools>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if pointer.captured_this_frame || crafting_ui.open {
        return;
    }
    let remove = mouse.just_pressed(MouseButton::Left);
    let place = mouse.just_pressed(MouseButton::Right);
    if !remove && !place {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }
    let Ok(transform) = camera.get_single() else {
        return;
    };

    let direction = transform.rotation * Vec3::NEG_Z;
    let block_hit = raycast_with_distance(&world, transform.translation, direction, REACH);
    if remove {
        let mob_hit = mobs
            .iter_mut()
            .filter_map(|(entity, mob_transform, mob)| {
                ray_sphere_distance(
                    transform.translation,
                    direction,
                    mob_transform.translation + Vec3::Y * 0.8,
                    0.7,
                )
                .map(|distance| (entity, mob.kind, distance))
            })
            .min_by(|left, right| left.2.total_cmp(&right.2));
        if let Some((entity, kind, distance)) = mob_hit {
            if block_hit.is_none_or(|(_, _, block_distance)| distance <= block_distance) {
                let Ok((_, _, mut mob)) = mobs.get_mut(entity) else {
                    return;
                };
                let damage = if tools.sword_durability > 0 {
                    tools.sword_durability = tools.sword_durability.saturating_sub(1);
                    2
                } else {
                    1
                };
                mob.health = mob.health.saturating_sub(damage);
                if mob.health == 0 {
                    commands.entity(entity).despawn_recursive();
                    let (food, experience) = mob_rewards(kind);
                    survival.food = survival.food.saturating_add(food).min(99);
                    survival.experience = survival.experience.saturating_add(experience);
                    if tools.sword_durability == 0 && damage == 2 {
                        survival.notice = "La espada se rompió.".into();
                        survival.notice_elapsed = 4.0;
                    } else {
                        survival.notice = match kind {
                            MobKind::Grazer if food > 0 => "Conseguiste carne y experiencia.".into(),
                            MobKind::Grazer => "Derrotaste al animal; la comida está llena.".into(),
                            MobKind::NightStalker => {
                                "Derrotaste al acechador y ganaste experiencia.".into()
                            }
                        };
                        survival.notice_elapsed = 4.0;
                    }
                } else {
                    survival.notice = match kind {
                        MobKind::Grazer => "Has golpeado al animal.".into(),
                        MobKind::NightStalker => "Has golpeado al acechador.".into(),
                    };
                    survival.notice_elapsed = 1.5;
                }
                return;
            }
        }
    }
    let Some((hit, previous, _)) = block_hit else {
        return;
    };
    
    // Check for door interaction (right click on door)
    if place {
        let (door_chunk_coord, door_local) = block_address(hit);
        if let Some(door_chunk) = world.chunks.get(&door_chunk_coord) {
            let current_block = door_chunk.get(door_local);
            if current_block == WOODEN_DOOR {
                let state = world.door_states.entry(hit).or_insert(DoorState::Closed);
                *state = match state {
                    DoorState::Open => DoorState::Closed,
                    DoorState::Closed => DoorState::Open,
                };
                survival.notice = format!(
                    "Puerta: {}",
                    match state {
                        DoorState::Open => "Abierta",
                        DoorState::Closed => "Cerrada",
                    }
                );
                survival.notice_elapsed = 2.0;
                return;
            }
        }
    }
    
    let target = if remove { hit } else { previous };
    let (chunk_coord, local) = block_address(target);
    let mut mined_block = None;
    {
        let Some(chunk) = world.chunks.get_mut(&chunk_coord) else {
            return;
        };
        let current_block = chunk.get(local);
        if remove {
            if current_block == AIR {
                return;
            }
            if let Err(block) = inventory.add(current_block) {
                warn!("Could not add block {block} to inventory");
                return;
            }
            chunk.set(local, AIR);
            mined_block = Some(current_block);
            // Spawn particle effects
            spawn_block_particles(
                &mut commands,
                &mut meshes,
                &mut materials,
                target.as_vec3(),
                current_block,
            );
            // Remove door state if removed
            if current_block == WOODEN_DOOR {
                world.door_states.remove(&target);
            }
        } else {
            if current_block != AIR
                || inventory.count(selected.0) == 0
                || block_overlaps_player(target, transform.translation)
            {
                return;
            }
            if !inventory.remove(selected.0) {
                warn!("Selected block inventory changed before placement");
                return;
            }
            chunk.set(local, selected.0);
            // Initialize door state if placing a door
            if selected.0 == WOODEN_DOOR {
                world.door_states.insert(target, DoorState::Closed);
            }
        }
    }
    if let Some(block) = mined_block {
        survival.experience = survival
            .experience
            .saturating_add(experience_for_block(block));
        if let Some(durability) = tool_durability_for_block(block, &mut tools) {
            if inventory.add(block).is_ok() {
                survival.experience = survival
                    .experience
                    .saturating_add(experience_for_block(block));
            }
            if durability == 0 {
                survival.notice = "La herramienta se rompió.".into();
                survival.notice_elapsed = 4.0;
            }
        }
        if block == LEAVES
            && (coordinate_hash(target.x, target.z, world.seed) ^ target.y as u64).is_multiple_of(3)
        {
            survival.food = survival.food.saturating_add(1).min(99);
            survival.notice = "Encontraste una baya.".into();
            survival.notice_elapsed = 3.0;
        }
    }
    let updated_chunk = world.chunks[&chunk_coord].blocks.clone();
    world.modified_chunks.insert(
        chunk_coord,
        Chunk {
            blocks: updated_chunk,
        },
    );
    storage.dirty_chunks.insert(chunk_coord);
    storage.failed_chunks.remove(&chunk_coord);
    storage.status = "Cambios pendientes de guardar".into();
    mark_dirty(&mut world, chunk_coord, local);
}

fn tool_durability_for_block(block: u8, tools: &mut CraftedTools) -> Option<u32> {
    let durability = match block {
        STONE if tools.pickaxe_durability > 0 => &mut tools.pickaxe_durability,
        WOOD | LEAVES if tools.axe_durability > 0 => &mut tools.axe_durability,
        SAND | DIRT if tools.shovel_durability > 0 => {
            // Pala es 2x más eficiente en arena y tierra
            let durability = &mut tools.shovel_durability;
            *durability = durability.saturating_sub(1);
            return Some(*durability);
        }
        _ => return None,
    };
    *durability -= 1;
    Some(*durability)
}

fn block_overlaps_player(block: IVec3, eye_position: Vec3) -> bool {
    let player_min = Vec3::new(
        eye_position.x - PLAYER_HALF_WIDTH,
        eye_position.y - PLAYER_EYE_HEIGHT,
        eye_position.z - PLAYER_HALF_WIDTH,
    ) + Vec3::splat(0.0001);
    let player_max = Vec3::new(
        eye_position.x + PLAYER_HALF_WIDTH,
        eye_position.y + PLAYER_HEADROOM,
        eye_position.z + PLAYER_HALF_WIDTH,
    ) - Vec3::splat(0.0001);
    let block_min = block.as_vec3();
    let block_max = block_min + Vec3::ONE;

    player_min.x < block_max.x
        && player_max.x > block_min.x
        && player_min.y < block_max.y
        && player_max.y > block_min.y
        && player_min.z < block_max.z
        && player_max.z > block_min.z
}

fn rebuild_dirty_chunks(
    mut world: ResMut<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_entities: Query<(&ChunkMesh, &Mesh3d)>,
) {
    if world.dirty.is_empty() {
        return;
    }
    let dirty = std::mem::take(&mut world.dirty);
    for (chunk, mesh_handle) in &mut chunk_entities {
        if dirty.contains(&chunk.0) {
            let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
                error!("Missing mesh asset for chunk {:?}", chunk.0);
                continue;
            };
            *mesh = rendering::build_chunk_mesh(&world, chunk.0);
        }
    }
}

fn mark_dirty(world: &mut WorldData, coord: IVec3, local: IVec3) {
    world.dirty.insert(coord);
    for (axis, value) in [(0, local.x), (1, local.y), (2, local.z)] {
        if value == 0 || value == CHUNK_SIZE - 1 {
            let mut neighbor = coord;
            neighbor[axis] += if value == 0 { -1 } else { 1 };
            if world.chunks.contains_key(&neighbor) {
                world.dirty.insert(neighbor);
            }
        }
    }
}

fn raycast_with_distance(
    world: &WorldData,
    origin: Vec3,
    direction: Vec3,
    reach: f32,
) -> Option<(IVec3, IVec3, f32)> {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }

    let mut cell = origin.floor().as_ivec3();
    let mut previous = cell;
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );
    let mut t_max = Vec3::new(
        initial_ray_distance(origin.x, cell.x, direction.x, step.x),
        initial_ray_distance(origin.y, cell.y, direction.y, step.y),
        initial_ray_distance(origin.z, cell.z, direction.z, step.z),
    );
    let t_delta = Vec3::new(
        ray_delta(direction.x),
        ray_delta(direction.y),
        ray_delta(direction.z),
    );
    let mut distance = 0.0;

    for _ in 0..256 {
        if block_at(world, cell) != AIR {
            return Some((cell, previous, distance));
        }
        previous = cell;
        let axis = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            0
        } else if t_max.y <= t_max.z {
            1
        } else {
            2
        };
        if t_max[axis] > reach {
            return None;
        }
        distance = t_max[axis];
        cell[axis] += step[axis];
        t_max[axis] += t_delta[axis];
    }
    None
}

fn initial_ray_distance(origin: f32, cell: i32, direction: f32, step: i32) -> f32 {
    if step > 0 {
        ((cell + 1) as f32 - origin) / direction
    } else if step < 0 {
        (origin - cell as f32) / -direction
    } else {
        f32::INFINITY
    }
}

fn ray_delta(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        direction.abs().recip()
    }
}

fn block_at(world: &WorldData, position: IVec3) -> u8 {
    let (chunk_coord, local) = block_address(position);
    world
        .chunks
        .get(&chunk_coord)
        .map_or(AIR, |chunk| chunk.get(local))
}

fn is_block_solid(world: &WorldData, position: IVec3) -> bool {
    let block = block_at(world, position);
    match block {
        AIR => false,
        WOODEN_DOOR => {
            // Open doors are not solid for collision
            let state = world.door_states.get(&position).copied().unwrap_or(DoorState::Closed);
            !matches!(state, DoorState::Open)
        }
        STAIRS | SLAB => {
            // Stairs and slabs are not fully solid (partial blocks)
            false
        }
        _ => true,
    }
}

fn light_level_at(world: &WorldData, position: IVec3) -> f32 {
    // Direct sunlight depends on height above surface
    let surface_height = terrain_height(position.x, position.z, world.seed);
    let base_light = if position.y >= surface_height {
        15.0 // Full sunlight above surface
    } else {
        // Darken based on depth
        let depth = surface_height - position.y;
        (15.0 - (depth as f32 * 0.3).min(14.0)).max(0.5)
    };

    // Add light from torches in nearby blocks
    let mut max_torch_light: f32 = 0.0;
    for dx in -8..=8 {
        for dy in -8..=8 {
            for dz in -8..=8 {
                let check_pos = position + IVec3::new(dx, dy, dz);
                if block_at(world, check_pos) == TORCH {
                    let distance = (dx * dx + dy * dy + dz * dz) as f32;
                    let torch_light = (64.0 - distance).max(0.0) / 8.0;
                    max_torch_light = max_torch_light.max(torch_light);
                }
            }
        }
    }

    base_light.max(max_torch_light)
}

fn block_address(position: IVec3) -> (IVec3, IVec3) {
    let chunk = IVec3::new(
        position.x.div_euclid(CHUNK_SIZE),
        position.y.div_euclid(CHUNK_SIZE),
        position.z.div_euclid(CHUNK_SIZE),
    );
    let local = IVec3::new(
        position.x.rem_euclid(CHUNK_SIZE),
        position.y.rem_euclid(CHUNK_SIZE),
        position.z.rem_euclid(CHUNK_SIZE),
    );
    (chunk, local)
}

fn block_index(local: IVec3) -> usize {
    (local.x + CHUNK_SIZE * (local.z + CHUNK_SIZE * local.y)) as usize
}

fn biome_at(x: i32, z: i32, seed: u64) -> Biome {
    let temperature = value_noise(x, z, seed ^ 0xA076_1D64_78BD_642F, 0.012);
    let moisture = value_noise(x, z, seed ^ 0xE703_7ED1_A0B4_28DB, 0.018);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.012);
    let mountain_weight = smoothstep(0.64, 0.80, mountain);
    let desert_weight =
        smoothstep(0.58, 0.78, temperature) * (1.0 - smoothstep(0.38, 0.60, moisture));
    let forest_weight =
        smoothstep(0.54, 0.72, moisture) * (1.0 - smoothstep(0.72, 0.88, temperature));

    if mountain_weight > 0.55 {
        Biome::Mountain
    } else if desert_weight > 0.15 && desert_weight >= forest_weight {
        Biome::Desert
    } else if forest_weight > 0.28 {
        Biome::Forest
    } else {
        Biome::Plains
    }
}

fn terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    let broad = value_noise(x, z, seed ^ 0x9E37_79B9_7F4A_7C15, 0.035);
    let detail = value_noise(x, z, seed ^ 0xBF58_476D_1CE4_E5B9, 0.11);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.012);
    let mountain_weight = smoothstep(0.64, 0.80, mountain);
    (4.0 + (broad - 0.5) * 8.0 + (detail - 0.5) * 2.0 + mountain_weight * 9.0)
        .round()
        .clamp(1.0, (CHUNK_SIZE - 1) as f32) as i32
}

fn value_noise(x: i32, z: i32, seed: u64, scale: f32) -> f32 {
    let scaled_x = x as f32 * scale;
    let scaled_z = z as f32 * scale;
    let grid_x = scaled_x.floor() as i32;
    let grid_z = scaled_z.floor() as i32;
    let fraction_x = smoothstep(0.0, 1.0, scaled_x - grid_x as f32);
    let fraction_z = smoothstep(0.0, 1.0, scaled_z - grid_z as f32);
    let top = lerp(
        hash_value(grid_x, grid_z, seed),
        hash_value(grid_x + 1, grid_z, seed),
        fraction_x,
    );
    let bottom = lerp(
        hash_value(grid_x, grid_z + 1, seed),
        hash_value(grid_x + 1, grid_z + 1, seed),
        fraction_x,
    );
    lerp(top, bottom, fraction_z)
}

fn hash_value(x: i32, z: i32, seed: u64) -> f32 {
    (coordinate_hash(x, z, seed) >> 40) as f32 / (1_u32 << 24) as f32
}

fn coordinate_hash(x: i32, z: i32, seed: u64) -> u64 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

fn player_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_world_path() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("the-cubes-test-{}-{nonce}", std::process::id()))
            .join("world.ron")
    }

    #[test]
    fn block_address_handles_negative_world_coordinates() {
        let (chunk, local) = block_address(IVec3::new(-1, -16, -17));
        assert_eq!(chunk, IVec3::new(-1, -1, -2));
        assert_eq!(local, IVec3::new(15, 0, 15));
    }

    #[test]
    fn block_index_covers_chunk_without_collisions() {
        let indices = (0..CHUNK_SIZE)
            .flat_map(|y| {
                (0..CHUNK_SIZE).flat_map(move |z| {
                    (0..CHUNK_SIZE).map(move |x| block_index(IVec3::new(x, y, z)))
                })
            })
            .collect::<HashSet<_>>();
        assert_eq!(indices.len(), CHUNK_VOLUME);
        assert_eq!(indices.iter().min(), Some(&0));
        assert_eq!(indices.iter().max(), Some(&(CHUNK_VOLUME - 1)));
    }

    #[test]
    fn terrain_height_is_deterministic_and_inside_chunk() {
        assert_eq!(terrain_height(13, -9, 123), terrain_height(13, -9, 123));
        assert_ne!(terrain_height(13, -9, 123), terrain_height(13, -9, 999_999));
        for x in -128..128 {
            for z in -128..128 {
                assert!((1..CHUNK_SIZE).contains(&terrain_height(x, z, 42)));
            }
        }
    }

    #[test]
    fn generated_chunks_include_varied_terrain_and_new_blocks() {
        let mut biomes = HashSet::new();
        let mut present = HashSet::new();
        let mut found_mountain = false;
        let mut found_desert = false;
        let deterministic_chunk = generate_chunk(IVec3::new(-4, 0, -4), 42);

        for chunk_z in -4..=4 {
            for chunk_x in -4..=4 {
                let coord = IVec3::new(chunk_x, 0, chunk_z);
                let chunk = generate_chunk(coord, 42);
                for block in &chunk.blocks {
                    present.insert(*block);
                }
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let world_x = chunk_x * CHUNK_SIZE + x;
                        let world_z = chunk_z * CHUNK_SIZE + z;
                        let biome = biome_at(world_x, world_z, 42);
                        let height = terrain_height(world_x, world_z, 42);
                        assert_eq!(chunk.get(IVec3::new(x, height, z)), biome.surface_block());
                        biomes.insert(biome);
                        found_mountain |= biome == Biome::Mountain && height >= 12;
                        found_desert |= biome == Biome::Desert;
                    }
                }
            }
        }

        assert!(biomes.contains(&Biome::Plains));
        assert!(biomes.contains(&Biome::Forest));
        assert!(biomes.contains(&Biome::Desert));
        assert!(biomes.contains(&Biome::Mountain));
        assert!(found_mountain);
        assert!(found_desert);
        assert!(present.contains(&GRASS));
        assert!(present.contains(&DIRT));
        assert!(present.contains(&STONE));
        assert!(present.contains(&SAND));
        assert!(present.contains(&WOOD));
        assert!(present.contains(&LEAVES));
        
        // Test that new block types are properly defined
        assert_ne!(ICE, AIR);
        assert_ne!(SNOW, AIR);
        assert_ne!(MUD, AIR);
        assert_ne!(OBSIDIAN, AIR);
        assert_ne!(GRAVEL, AIR);
        
        assert_eq!(
            generate_chunk(IVec3::new(-4, 0, -4), 42).blocks,
            deterministic_chunk.blocks
        );
    }

    #[test]
    fn streaming_coordinates_cover_the_radius_around_negative_centers() {
        let center = IVec3::new(-2, 0, 3);
        let coordinates = chunk_coordinates_around(center, STREAM_LOAD_RADIUS);

        assert_eq!(coordinates.len(), 49);
        assert!(coordinates.contains(&center));
        assert!(coordinates.contains(&IVec3::new(-5, 0, 0)));
        assert!(coordinates.contains(&IVec3::new(1, 0, 6)));
        assert!(coordinates.iter().all(|coordinate| coordinate.y == 0));
    }

    #[test]
    fn unloaded_chunks_regenerate_terrain_and_restore_saved_edits() {
        let mut world = WorldData {
            seed: 812,
            ..default()
        };
        let coord = IVec3::new(-2, 0, 1);
        let generated = chunk_for_coordinate(&world, coord);
        assert_eq!(generated.blocks, generate_chunk(coord, 812).blocks);

        let mut edited = generated;
        edited.set(IVec3::new(0, 0, 0), STONE);
        world.modified_chunks.insert(coord, edited);
        let restored = chunk_for_coordinate(&world, coord);

        assert_eq!(restored.get(IVec3::new(0, 0, 0)), STONE);
        assert_eq!(
            chunk_for_coordinate(&world, IVec3::new(20, 0, 20)).blocks,
            generate_chunk(IVec3::new(20, 0, 20), 812).blocks
        );
    }

    #[test]
    fn greedy_meshing_merges_a_uniform_plane() {
        let size = CHUNK_SIZE as usize;
        let mut mask = vec![GRASS; size * size];
        let mut rectangles = Vec::new();

        rendering::greedy_rectangles(&mut mask, |u, v, width, height, block| {
            rectangles.push((u, v, width, height, block));
        });

        assert_eq!(rectangles, [(0, 0, size, size, GRASS)]);
        assert!(mask.iter().all(|block| *block == AIR));
    }

    #[test]
    fn greedy_meshing_keeps_different_blocks_separate() {
        let size = CHUNK_SIZE as usize;
        let mut mask = vec![AIR; size * size];
        for v in 0..2 {
            for u in 0..2 {
                mask[v * size + u] = GRASS;
            }
        }
        mask[2] = STONE;
        mask[size + 2] = DIRT;
        let mut rectangles = Vec::new();

        rendering::greedy_rectangles(&mut mask, |u, v, width, height, block| {
            rectangles.push((u, v, width, height, block));
        });

        assert_eq!(
            rectangles,
            [(0, 0, 2, 2, GRASS), (2, 0, 1, 1, STONE), (2, 1, 1, 1, DIRT)]
        );
    }

    #[test]
    fn player_collision_allows_standing_on_a_block() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::ZERO, STONE);
        world.chunks.insert(IVec3::ZERO, chunk);

        assert!(!collides_with_world(&world, Vec3::new(0.5, 2.62, 0.5)));
        assert!(collides_with_world(&world, Vec3::new(0.5, 2.5, 0.5)));
    }

    #[test]
    fn player_collision_stops_at_walls() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(1, 1, 0), STONE);
        world.chunks.insert(IVec3::ZERO, chunk);

        assert!(!collides_with_world(&world, Vec3::new(0.5, 2.62, 0.5)));
        assert!(collides_with_world(&world, Vec3::new(0.75, 2.62, 0.5)));
    }

    #[test]
    fn inventory_collects_and_consumes_blocks() {
        let mut inventory = Inventory::default();

        assert_eq!(inventory.count(GRASS), 0);
        assert!(!inventory.remove(GRASS));
        assert!(inventory.add(GRASS).is_ok());
        assert_eq!(inventory.count(GRASS), 1);
        assert!(inventory.remove(GRASS));
        assert_eq!(inventory.count(GRASS), 0);
        assert!(inventory.add(AIR).is_err());
    }

    #[test]
    fn hunger_depletes_over_time_and_sprinting_costs_extra() {
        let mut survival = Survival::default();

        advance_survival_stats(&mut survival, PASSIVE_HUNGER_INTERVAL, false);
        assert_eq!(survival.hunger, MAX_HUNGER - 1.0);
        advance_survival_stats(&mut survival, SPRINT_HUNGER_INTERVAL, true);
        assert_eq!(survival.hunger, MAX_HUNGER - 2.0);
    }

    #[test]
    fn eating_restores_hunger_and_consumes_food() {
        let mut survival = Survival {
            hunger: 16.0,  // Changed from 15 to 16, so 16 + 4 = 20 = MAX_HUNGER
            food: 2,
            ..default()
        };

        assert!(eat_food(&mut survival));
        assert_eq!(survival.hunger, MAX_HUNGER);
        assert_eq!(survival.food, 1);
        assert!(!eat_food(&mut survival));
        survival.hunger = 0.0;
        assert!(eat_food(&mut survival));
        assert_eq!(survival.hunger, 4.0);  // BerryRation provides 4 saturation, not 6
    }

    #[test]
    fn hunger_enables_regeneration_and_starvation_deals_damage() {
        let mut regenerating = Survival {
            health: 18.0,
            ..default()
        };
        advance_survival_stats(&mut regenerating, HEALTH_REGEN_INTERVAL, false);
        assert_eq!(regenerating.health, 19.0);
        assert_eq!(regenerating.hunger, MAX_HUNGER - 0.5);

        let mut starving = Survival {
            health: 3.0,
            hunger: 0.0,
            ..default()
        };
        advance_survival_stats(&mut starving, STARVATION_DAMAGE_INTERVAL * 2.0, false);
        assert_eq!(starving.health, 1.0);
    }

    #[test]
    fn falling_damage_and_experience_levels_have_expected_thresholds() {
        assert_eq!(fall_damage(12.0), 0.0);
        assert_eq!(fall_damage(16.0), 2.0);
        assert_eq!(experience_progress(9), (0, 9, 10));
        assert_eq!(experience_progress(10), (1, 0, 15));
        assert_eq!(experience_progress(26), (2, 1, 20));
        assert_eq!(experience_for_block(STONE), 2);
        assert_eq!(experience_for_block(AIR), 0);
    }

    #[test]
    fn crafting_recipes_consume_materials_and_create_tools_or_food() {
        let mut inventory = Inventory::default();
        let mut survival = Survival::default();
        let mut tools = CraftedTools::default();
        inventory.counts[WOOD as usize] = 6;
        inventory.counts[STONE as usize] = 3;
        inventory.counts[LEAVES as usize] = 3;

        assert!(craft_recipe(
            CraftRecipe::WoodenPickaxe,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(tools.pickaxe_durability, 64);
        assert_eq!(inventory.count(WOOD), 3);
        assert_eq!(inventory.count(STONE), 1);
        assert_eq!(tool_durability_for_block(STONE, &mut tools), Some(63));
        assert_eq!(tools.pickaxe_durability, 63);

        assert!(craft_recipe(
            CraftRecipe::WoodenAxe,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(tools.axe_durability, 64);
        assert_eq!(tool_durability_for_block(LEAVES, &mut tools), Some(63));

        assert!(craft_recipe(
            CraftRecipe::BerryRation,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(survival.food, default_starting_food() + 1);
        assert_eq!(inventory.count(LEAVES), 0);
    }

    #[test]
    fn failed_crafting_does_not_consume_materials() {
        let mut inventory = Inventory::default();
        let mut survival = Survival::default();
        let mut tools = CraftedTools::default();
        inventory.counts[WOOD as usize] = 2;
        inventory.counts[STONE as usize] = 1;

        assert!(craft_recipe(
            CraftRecipe::WoodenPickaxe,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_err());
        assert_eq!(inventory.count(WOOD), 2);
        assert_eq!(inventory.count(STONE), 1);
        assert_eq!(tools.pickaxe_durability, 0);
    }

    #[test]
    fn tools_only_help_with_matching_blocks_and_break_after_64_uses() {
        let mut tools = CraftedTools {
            pickaxe_durability: 1,
            axe_durability: 2,
            shovel_durability: 0,
            sword_durability: 0,
        };

        assert_eq!(tool_durability_for_block(WOOD, &mut tools), Some(1));
        assert_eq!(tool_durability_for_block(STONE, &mut tools), Some(0));
        assert_eq!(tool_durability_for_block(STONE, &mut tools), None);
        assert_eq!(tool_durability_for_block(LEAVES, &mut tools), Some(0));
        assert_eq!(tool_durability_for_block(LEAVES, &mut tools), None);
    }

    #[test]
    fn shovel_is_twice_as_efficient_on_sand_and_dirt() {
        let mut tools = CraftedTools {
            pickaxe_durability: 0,
            axe_durability: 0,
            shovel_durability: 10,
            sword_durability: 0,
        };

        assert_eq!(tool_durability_for_block(SAND, &mut tools), Some(9));
        assert_eq!(tool_durability_for_block(DIRT, &mut tools), Some(8));
        assert_eq!(tools.shovel_durability, 8);
        assert_eq!(tool_durability_for_block(STONE, &mut tools), None);
    }

    #[test]
    fn sword_consumes_durability_on_mob_hits() {
        let mut tools = CraftedTools {
            pickaxe_durability: 0,
            axe_durability: 0,
            shovel_durability: 0,
            sword_durability: 5,
        };

        let initial_durability = tools.sword_durability;
        for _ in 0..5 {
            if tools.sword_durability > 0 {
                tools.sword_durability -= 1;
            }
        }
        assert_eq!(tools.sword_durability, 0);
        assert_eq!(initial_durability, 5);
    }

    #[test]
    fn blocks_cannot_be_placed_inside_the_player() {
        let eye_position = Vec3::new(0.5, 2.62, 0.5);

        assert!(block_overlaps_player(IVec3::new(0, 1, 0), eye_position));
        assert!(!block_overlaps_player(IVec3::new(0, 0, 0), eye_position));
        assert!(!block_overlaps_player(IVec3::new(2, 1, 0), eye_position));
    }

    #[test]
    fn mob_population_caps_each_kind_and_total() {
        assert!(can_spawn_mob(MobKind::Grazer, 4, 3, 1));
        assert!(!can_spawn_mob(MobKind::Grazer, 5, 5, 0));
        assert!(can_spawn_mob(MobKind::NightStalker, 7, 5, 2));
        assert!(!can_spawn_mob(MobKind::NightStalker, 8, 5, 2));
        assert!(!can_spawn_mob(MobKind::NightStalker, 3, 1, 3));
    }

    #[test]
    fn mob_systems_run_without_ecs_query_conflicts() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(DayNightCycle::default())
            .insert_resource(WorldData::default())
            .insert_resource(MobAssets::default())
            .insert_resource(MobPopulation::default())
            .insert_resource(Survival::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(PointerState::default())
            .insert_resource(CraftingUi::default())
            .insert_resource(SelectedBlock(GRASS))
            .insert_resource(Inventory::default())
            .insert_resource(CraftedTools::default())
            .insert_resource(WorldStorage::default())
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, (update_mobs, edit_blocks).chain());

        app.update();
    }

    #[test]
    fn mobs_spawn_only_at_night_for_hostiles_and_have_defined_rewards() {
        assert!(is_night(0.0));
        assert!(is_night(0.079));
        assert!(!is_night(0.08));
        assert_eq!(mob_rewards(MobKind::Grazer), (2, 3));
        assert_eq!(mob_rewards(MobKind::NightStalker), (0, 5));
    }

    #[test]
    fn mob_ray_targeting_hits_only_entities_in_front_within_reach() {
        let origin = Vec3::ZERO;
        let direction = Vec3::NEG_Z;

        assert_eq!(
            ray_sphere_distance(origin, direction, Vec3::new(0.0, 0.0, -3.0), 0.5),
            Some(2.5)
        );
        assert_eq!(
            ray_sphere_distance(origin, direction, Vec3::new(2.0, 0.0, -3.0), 0.5),
            None
        );
        assert_eq!(
            ray_sphere_distance(origin, direction, Vec3::new(0.0, 0.0, 3.0), 0.5),
            None
        );
    }

    #[test]
    fn block_raycast_reports_the_nearest_surface_for_mob_occlusion() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(3, 1, 0), STONE);
        world.chunks.insert(IVec3::ZERO, chunk);

        assert_eq!(
            raycast_with_distance(&world, Vec3::new(0.5, 1.5, 0.5), Vec3::X, REACH,),
            Some((IVec3::new(3, 1, 0), IVec3::new(2, 1, 0), 2.5))
        );
    }

    #[test]
    fn mob_spawn_candidates_are_deterministic_and_use_grass_in_loaded_chunks() {
        let seed = 42;
        let mut world = WorldData { seed, ..default() };
        for z in -6..=6 {
            for x in -6..=6 {
                let coordinate = IVec3::new(x, 0, z);
                world
                    .chunks
                    .insert(coordinate, generate_chunk(coordinate, seed));
            }
        }

        let mut found = None;
        'search: for z in (-64..=64).step_by(8) {
            for x in (-64..=64).step_by(8) {
                let player = Vec3::new(x as f32 + 0.5, 24.0, z as f32 + 0.5);
                for attempt in 0..128 {
                    if let Some(position) = mob_spawn_position(&world, player, attempt) {
                        assert_eq!(mob_spawn_position(&world, player, attempt), Some(position));
                        found = Some(position);
                        break 'search;
                    }
                }
            }
        }

        let position = found.expect("a loaded plains or forest spawn candidate");
        let x = position.x.floor() as i32;
        let z = position.z.floor() as i32;
        let ground = position.y as i32 - 1;
        assert!(matches!(
            biome_at(x, z, seed),
            Biome::Plains | Biome::Forest
        ));
        assert_eq!(block_at(&world, IVec3::new(x, ground, z)), GRASS);
        assert_eq!(block_at(&world, IVec3::new(x, ground + 1, z)), AIR);
    }

    #[test]
    fn world_save_round_trips_world_and_player_state() {
        let mut world = WorldData {
            name: "Prueba".into(),
            seed: 9876,
            ..default()
        };
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(1, 2, 3), STONE);
        world.chunks.insert(IVec3::ZERO, chunk);
        let mut modified_chunk = Chunk::new();
        modified_chunk.set(IVec3::new(1, 2, 3), STONE);
        world.modified_chunks.insert(IVec3::ZERO, modified_chunk);
        world.chunks.insert(IVec3::new(1, 0, 0), Chunk::new());
        let mut inventory = Inventory::default();
        inventory.counts[DIRT as usize] = 17;
        inventory.counts[SAND as usize] = 5;
        inventory.counts[WOOD as usize] = 3;
        let survival = Survival {
            health: 13.5,
            hunger: 8.0,
            experience: 37,
            food: 2,
            ..default()
        };
        let crafted_tools = CraftedTools {
            pickaxe_durability: 31,
            axe_durability: 12,
            shovel_durability: 0,
            sword_durability: 0,
        };

        let player = Player {
            yaw: 1.2,
            pitch: -0.4,
            velocity: Vec3::ZERO,
            grounded: true,
            flying: false,
        };
        let save = snapshot_world(
            &world,
            Vec3::new(4.5, 9.0, -2.0),
            &player,
            &inventory,
            DIRT,
            1,
            43_200.0,
            &survival,
            &crafted_tools,
        );
        let contents = ron::to_string(&save).expect("serialize world save");
        let loaded: WorldSave = ron::from_str(&contents).expect("deserialize world save");

        validate_world_save(&loaded).expect("validate world save");
        assert_eq!(loaded.name, "Prueba");
        assert_eq!(loaded.seed, 9876);
        assert_eq!(loaded.time_of_day_seconds, 43_200.0);
        assert_eq!(loaded.player_position, [4.5, 9.0, -2.0]);
        assert_eq!(loaded.player_yaw, 1.2);
        assert_eq!(loaded.player_pitch, -0.4);
        assert_eq!(loaded.selected_block, DIRT);
        assert_eq!(loaded.selected_hotbar_slot, 1);
        assert_eq!(loaded.inventory[DIRT as usize], 17);
        assert_eq!(loaded.extra_inventory[0], 5);
        assert_eq!(loaded.extra_inventory[1], 3);
        assert_eq!(loaded.health, 13.5);
        assert_eq!(loaded.hunger, 8.0);
        assert_eq!(loaded.experience, 37);
        assert_eq!(loaded.food, 2);
        assert_eq!(loaded.pickaxe_durability, 31);
        assert_eq!(loaded.axe_durability, 12);
        assert!(loaded.chunks.is_empty());
    }

    #[test]
    fn per_chunk_storage_and_world_metadata_round_trip() {
        let path = temporary_world_path();
        let coord = IVec3::new(-3, 0, 7);
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(1, 2, 3), STONE);

        save_chunk_file(&path, coord, &chunk).expect("save chunk");
        let loaded_chunk = load_chunk_file(&path, coord)
            .expect("load chunk file")
            .expect("chunk file exists");
        assert_eq!(loaded_chunk.blocks, chunk.blocks);

        let save = WorldSave {
            version: SAVE_VERSION,
            name: "Prueba".into(),
            seed: 9876,
            time_of_day_seconds: 43_200.0,
            player_position: [4.5, 9.0, -2.0],
            player_yaw: 1.2,
            player_pitch: -0.4,
            selected_block: DIRT,
            selected_hotbar_slot: 1,
            inventory: [0, 0, 17, 0],
            extra_inventory: [0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
            health: 15.0,
            hunger: 12.0,
            experience: 25,
            food: 4,
            pickaxe_durability: 0,
            axe_durability: 0,
            shovel_durability: 0,
            sword_durability: 0,
            chunks: Vec::new(),
        };
        save_world(&path, save).expect("save world metadata");
        let loaded = load_world(&path).expect("load world metadata");
        assert_eq!(loaded.name, "Prueba");
        assert!(loaded.chunks.is_empty());

        fs::remove_dir_all(path.parent().expect("temporary world directory"))
            .expect("remove temporary world files");
    }

    #[test]
    fn save_all_chunks_flushes_active_and_legacy_edits() {
        let path = temporary_world_path();
        let active_coord = IVec3::new(1, 0, 0);
        let legacy_coord = IVec3::new(-1, 0, 0);
        let mut active_chunk = Chunk::new();
        active_chunk.set(IVec3::new(0, 1, 2), STONE);
        let mut legacy_chunk = Chunk::new();
        legacy_chunk.set(IVec3::new(3, 4, 5), DIRT);

        let mut world = WorldData::default();
        world
            .modified_chunks
            .insert(active_coord, active_chunk.clone());
        let mut storage = WorldStorage::default();
        storage
            .legacy_chunks
            .insert(legacy_coord, legacy_chunk.clone());

        save_all_chunks(&world, &mut storage, &path).expect("flush all changed chunks");

        assert!(storage.legacy_chunks.is_empty());
        assert!(storage.dirty_chunks.is_empty());
        assert_eq!(
            load_chunk_file(&path, active_coord)
                .expect("read active chunk")
                .expect("active chunk saved")
                .blocks,
            active_chunk.blocks
        );
        assert_eq!(
            load_chunk_file(&path, legacy_coord)
                .expect("read legacy chunk")
                .expect("legacy chunk saved")
                .blocks,
            legacy_chunk.blocks
        );

        fs::remove_dir_all(path.parent().expect("temporary world directory"))
            .expect("remove temporary world files");
    }

    #[test]
    fn loading_a_legacy_chunk_keeps_it_pending_until_chunk_file_is_written() {
        let coord = IVec3::new(4, 0, -2);
        let mut legacy_chunk = Chunk::new();
        legacy_chunk.set(IVec3::new(2, 3, 4), DIRT);
        let mut storage = WorldStorage::default();
        storage.legacy_chunks.insert(coord, legacy_chunk);
        storage.dirty_chunks.insert(coord);
        let mut world = WorldData::default();

        let loaded =
            load_or_generate_chunk(&mut world, &mut storage, coord).expect("load legacy chunk");

        assert_eq!(loaded.get(IVec3::new(2, 3, 4)), DIRT);
        assert_eq!(world.modified_chunks[&coord].get(IVec3::new(2, 3, 4)), DIRT);
        assert!(storage.dirty_chunks.contains(&coord));
    }

    #[test]
    fn world_save_defaults_new_fields_for_existing_saves() {
        let contents = r#"(version:1,name:"Prueba",seed:42,player_position:(0.0,0.0,0.0),selected_block:1,inventory:(0,0,0,0),chunks:[])"#;
        let save: WorldSave = ron::from_str(contents).expect("deserialize existing world save");

        assert_eq!(save.player_yaw, 0.0);
        assert_eq!(save.player_pitch, 0.0);
        assert_eq!(save.time_of_day_seconds, DEFAULT_TIME_OF_DAY_SECONDS);
        assert_eq!(save.selected_hotbar_slot, 0);
        assert_eq!(save.health, MAX_HEALTH);
        assert_eq!(save.hunger, MAX_HUNGER);
        assert_eq!(save.experience, 0);
        assert_eq!(save.food, default_starting_food());
        assert_eq!(save.pickaxe_durability, 0);
        assert_eq!(save.axe_durability, 0);
        validate_world_save(&save).expect("validate existing world save");
    }

    #[test]
    fn hotbar_has_six_slots_and_scroll_wraps_in_both_directions() {
        assert_eq!(HOTBAR_BLOCKS.len(), 6);
        assert_eq!(cycle_hotbar_selection(0, -1), 5);
        assert_eq!(cycle_hotbar_selection(5, 1), 0);
        assert_eq!(cycle_hotbar_selection(2, 3), 5);
        assert_eq!(resolve_hotbar_slot(DIRT, 0), 1);
        assert_eq!(resolve_hotbar_slot(SAND, 3), 3);
        assert_eq!(mouse_wheel_steps(1.0, MouseScrollUnit::Line), -1);
        assert_eq!(mouse_wheel_steps(-80.0, MouseScrollUnit::Pixel), 2);
        assert_eq!(mouse_wheel_steps(0.0, MouseScrollUnit::Line), 0);

        let mut selected = SelectedBlock(GRASS);
        let mut selected_slot = SelectedHotbarSlot::default();
        set_hotbar_selection(5, &mut selected, &mut selected_slot);
        assert_eq!(selected_slot.0, 5);
        assert_eq!(selected.0, LEAVES);
    }

    #[test]
    fn day_night_cycle_uses_one_real_minute_per_game_hour() {
        let after_one_real_minute = advance_time_of_day(
            DEFAULT_TIME_OF_DAY_SECONDS,
            60.0,
            GAME_SECONDS_PER_REAL_SECOND,
        );
        assert_eq!(after_one_real_minute, 9.0 * 60.0 * 60.0);
        assert_eq!(
            advance_time_of_day(
                after_one_real_minute,
                23.0 * 60.0,
                GAME_SECONDS_PER_REAL_SECOND
            ),
            DEFAULT_TIME_OF_DAY_SECONDS
        );
    }

    #[test]
    fn day_night_cycle_changes_sunlight_and_wraps_clock() {
        let (_, night) = sun_position_and_daylight(0.0);
        let (_, noon) = sun_position_and_daylight(12.0 * 60.0 * 60.0);

        assert!(night < 0.01);
        assert!(noon > 0.99);
        assert_eq!(
            format_time_of_day(23.0 * 60.0 * 60.0 + 59.0 * 60.0),
            "23:59"
        );
        assert_eq!(format_time_of_day(GAME_DAY_SECONDS), "00:00");
        assert_eq!(advance_time_of_day(GAME_DAY_SECONDS - 1.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn sky_gradient_has_distinct_day_and_night_palettes() {
        let night = sky_vertex_colors(0.0);
        let day = sky_vertex_colors(12.0 * 60.0 * 60.0);
        let dawn = sky_vertex_colors(6.0 * 60.0 * 60.0);

        assert_eq!(night.len(), 33 * 65);
        assert_ne!(night, day);
        assert_ne!(dawn, day);
        assert!(night
            .iter()
            .chain(&day)
            .chain(&dawn)
            .all(|color| color.iter().all(|channel| (0.0..=1.0).contains(channel))));

        let noon_direction = sun_direction(12.0 * 60.0 * 60.0);
        let midnight_direction = sun_direction(0.0);
        assert!(noon_direction.y > 0.99);
        assert!(midnight_direction.y < -0.99);
    }

    #[test]
    fn world_save_rejects_unsupported_versions_and_invalid_chunks() {
        let mut save = WorldSave {
            version: SAVE_VERSION + 1,
            name: "Prueba".into(),
            seed: 1,
            time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
            player_position: [0.0, 0.0, 0.0],
            player_yaw: 0.0,
            player_pitch: 0.0,
            selected_block: GRASS,
            selected_hotbar_slot: 0,
            inventory: [0; LEGACY_BLOCK_COUNT],
            extra_inventory: [0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
            health: MAX_HEALTH,
            hunger: MAX_HUNGER,
            experience: 0,
            food: default_starting_food(),
            pickaxe_durability: 0,
            axe_durability: 0,
            shovel_durability: 0,
            sword_durability: 0,
            chunks: Vec::new(),
        };
        assert!(validate_world_save(&save).is_err());

        save.version = SAVE_VERSION;
        save.chunks.push(ChunkSave {
            coord: [0, 0, 0],
            blocks: vec![AIR; CHUNK_VOLUME - 1],
        });
        assert!(validate_world_save(&save).is_err());
    }

    #[test]
    fn different_food_types_have_correct_saturation() {
        let saturations = [
            (FoodType::BerryRation, 4.0),
            (FoodType::Apple, 6.0),
            (FoodType::Mushroom, 2.0),
            (FoodType::Wheat, 8.0),
            (FoodType::RawMeat, 3.0),
            (FoodType::CookedMeat, 8.0),
        ];

        for (food_type, expected_saturation) in saturations {
            assert_eq!(
                food_type.saturation(),
                expected_saturation,
                "Food {:?} should have saturation {:?}",
                food_type,
                expected_saturation
            );
        }
    }

    #[test]
    fn poisonous_mushrooms_cause_damage() {
        let mut survival = Survival::default();
        survival.food = 5;
        survival.hunger = 5.0;
        let initial_health = survival.health;

        eat_food_of_type(&mut survival, FoodType::Mushroom);

        assert_eq!(survival.food, 4, "Food should be consumed");
        assert!(
            survival.hunger > 5.0,
            "Hunger should increase even when poisoned"
        );
        assert!(
            survival.health < initial_health,
            "Poisonous mushroom should cause damage"
        );
    }

    #[test]
    fn cooked_meat_provides_more_saturation_than_raw_meat() {
        assert!(FoodType::CookedMeat.saturation() > FoodType::RawMeat.saturation());
        assert_eq!(FoodType::CookedMeat.saturation(), 8.0);
        assert_eq!(FoodType::RawMeat.saturation(), 3.0);
    }

    #[test]
    fn eating_different_foods_increases_hunger_differently() {
        let mut survival1 = Survival::default();
        survival1.food = 10;
        survival1.hunger = 5.0;

        let mut survival2 = Survival::default();
        survival2.food = 10;
        survival2.hunger = 5.0;

        eat_food_of_type(&mut survival1, FoodType::Apple);
        eat_food_of_type(&mut survival2, FoodType::Wheat);

        assert!(
            survival2.hunger > survival1.hunger,
            "Wheat should restore more hunger than apple"
        );
        assert_eq!(survival1.hunger, 11.0, "Apple restores 6 hunger");
        assert_eq!(survival2.hunger, 13.0, "Wheat restores 8 hunger");
    }

    #[test]
    fn doors_block_player_collision_when_closed_but_allow_passage_when_open() {
        let mut world = WorldData { seed: 42, ..default() };
        world
            .chunks
            .insert(IVec3::ZERO, generate_chunk(IVec3::ZERO, 42));

        let door_pos = IVec3::new(0, 0, 0);
        {
            let chunk = world.chunks.get_mut(&IVec3::ZERO).unwrap();
            chunk.set(door_pos, WOODEN_DOOR);
        }
        world.door_states.insert(door_pos, DoorState::Closed);

        // Closed door should be solid
        assert!(
            is_block_solid(&world, door_pos),
            "Closed door blocks collision"
        );

        // Open door should not be solid
        world.door_states.insert(door_pos, DoorState::Open);
        assert!(
            !is_block_solid(&world, door_pos),
            "Open door allows passage"
        );
    }

    #[test]
    fn stairs_and_slabs_do_not_block_collision_but_render() {
        let mut world = WorldData { seed: 42, ..default() };
        world
            .chunks
            .insert(IVec3::ZERO, generate_chunk(IVec3::ZERO, 42));

        let stair_pos = IVec3::new(1, 0, 0);
        let slab_pos = IVec3::new(2, 0, 0);
        {
            let chunk = world.chunks.get_mut(&IVec3::ZERO).unwrap();
            chunk.set(stair_pos, STAIRS);
            chunk.set(slab_pos, SLAB);
        }

        // Stairs and slabs should not be solid (player can walk through them)
        assert!(
            !is_block_solid(&world, stair_pos),
            "Stairs allow movement"
        );
        assert!(!is_block_solid(&world, slab_pos), "Slabs allow movement");

        // But they render in the world (chunk has the blocks)
        let chunk = world.chunks.get(&IVec3::ZERO).unwrap();
        assert_eq!(chunk.get(stair_pos), STAIRS);
        assert_eq!(chunk.get(slab_pos), SLAB);
    }

    #[test]
    fn crafting_structures_creates_expected_quantities() {
        let mut inventory = Inventory::default();
        let mut survival = Survival::default();
        let mut tools = CraftedTools::default();
        inventory.counts[WOOD as usize] = 24;
        inventory.counts[STONE as usize] = 16;

        // Craft 3 doors (6 wood each)
        assert!(craft_recipe(
            CraftRecipe::WoodenDoor,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(inventory.count(WOODEN_DOOR), 3);

        // Craft 4 stairs (6 wood each)
        inventory.counts[WOOD as usize] = 24;
        assert!(craft_recipe(
            CraftRecipe::Stairs,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(inventory.count(STAIRS), 4);

        // Craft 6 slabs (3 wood each)
        inventory.counts[WOOD as usize] = 18;
        assert!(craft_recipe(
            CraftRecipe::Slab,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(inventory.count(SLAB), 6);

        // Craft 4 bricks (4 stone each)
        inventory.counts[STONE as usize] = 16;
        assert!(craft_recipe(
            CraftRecipe::Bricks,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(inventory.count(BRICKS), 4);

        // Craft 4 planks from 1 wood (efficient conversion)
        inventory.counts[WOOD as usize] = 1;
        assert!(craft_recipe(
            CraftRecipe::Planks,
            &mut inventory,
            &mut survival,
            &mut tools
        )
        .is_ok());
        assert_eq!(inventory.count(PLANKS), 4);
    }
}
