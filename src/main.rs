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

#[cfg(feature = "plains-audio")]
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use bevy::{
    image::ImageSampler,
    input::{
        keyboard::{Key, KeyboardInput},
        mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
    },
    window::{CursorGrabMode, MonitorSelection, PrimaryWindow, WindowMode, WindowPlugin},
};
use serde::{Deserialize, Serialize};

// Module declarations
mod blocks;
mod item_inventory;
mod mobs;
mod systems;
mod texture_layout;
mod world;

// Selective imports from modules
use blocks::*;
use item_inventory::{Inventory, Item, ItemStack, ToolKind, HOTBAR_SIZE, INVENTORY_SIZE};
use mobs::*;
use systems::*;
use world::*;

const WORLD_HEIGHT: i32 = 128;
const SEA_LEVEL: i32 = 48;
const TERRAIN_VERSION: u32 = 4;
const REACH: f32 = 6.0;
const PLAYER_HALF_WIDTH: f32 = 0.30;
const PLAYER_EYE_HEIGHT: f32 = 1.62;
const PLAYER_HEADROOM: f32 = 0.18;
const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 8.0;
const GROUND_ACCELERATION: f32 = 28.0;
const GROUND_FRICTION: f32 = 36.0;
const AIR_ACCELERATION: f32 = 6.0;
const AIR_DRAG: f32 = 1.2;
const FLIGHT_ACCELERATION: f32 = 42.0;
const FLIGHT_FRICTION: f32 = 30.0;
const GRAVITY: f32 = 24.0;
const JUMP_SPEED: f32 = 8.5;
const STEP_HEIGHT: f32 = 1.0;
const STEP_UP_SPEED: f32 = 7.0;
const JUMP_BUFFER_SECONDS: f32 = 0.12;
const COYOTE_TIME_SECONDS: f32 = 0.10;
const SAVE_VERSION: u32 = 3;
const STREAM_LOAD_RADIUS: i32 = 10;
const STREAM_UNLOAD_RADIUS: i32 = 11;
const STARTUP_LOAD_RADIUS: i32 = 3;
const STREAM_VERTICAL_RADIUS: i32 = 1;
const MAX_DIRTY_CHUNK_MESH_REBUILDS_PER_FRAME: usize = 8;
const MAX_CHUNK_UNLOADS_PER_FRAME: usize = 2;
const GAME_DAY_SECONDS: f32 = 24.0 * 60.0 * 60.0;
const GAME_SECONDS_PER_REAL_SECOND: f32 = 60.0;
const DEFAULT_TIME_OF_DAY_SECONDS: f32 = 8.0 * 60.0 * 60.0;
const SKY_RADIUS: f32 = 180.0;
const CELESTIAL_RADIUS: f32 = 120.0;
const STAR_RADIUS: f32 = 145.0;
const HOTBAR_SLOT_COUNT: usize = HOTBAR_SIZE;
const AUTOSAVE_INTERVAL_SECONDS: f32 = 60.0;
const MAX_HEALTH: f32 = 20.0;
const MAX_HUNGER: f32 = 20.0;
const MINING_CRACK_STAGES: usize = 8;
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

    /// Textures keep the same orientation in every block position.
    pub fn block_variant(_x: i32, _y: i32, _z: i32) -> u32 {
        0
    }

    /// Get upright UV coordinates in the atlas for one block face.
    /// Atlas is 16x16 grid, each tile is 1/16th of the texture
    /// Returns (u_min, v_min, u_max, v_max) in normalized coordinates [0, 1]
    pub fn block_uv(
        block: u8,
        face: usize,
        _variant: u32,
    ) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        // Map block type to atlas tile position
        // Special case: GRASS has different top and side textures
        let tile_id = block_texture(block, face);

        let columns = texture_layout::ATLAS_COLUMNS as usize;
        let tile_x = tile_id % columns;
        let tile_y = tile_id / columns;
        let uv_min_x = tile_x as f32 / columns as f32;
        let uv_max_x = (tile_x as f32 + 1.0) / columns as f32;
        let uv_min_y = tile_y as f32 / columns as f32;
        let uv_max_y = (tile_y as f32 + 1.0) / columns as f32;

        // Base UV corners: (bottom-left, bottom-right, top-right, top-left)
        let base_uv0 = [uv_min_x, uv_min_y];
        let base_uv1 = [uv_max_x, uv_min_y];
        let base_uv2 = [uv_max_x, uv_max_y];
        let base_uv3 = [uv_min_x, uv_max_y];

        // PNG/atlas V=0 is the top row. Map top vertices to the top of the
        // texture on every side, accounting for each face's vertex winding.
        match face {
            0 | 5 => (base_uv3, base_uv0, base_uv1, base_uv2),
            1 | 4 => (base_uv3, base_uv2, base_uv1, base_uv0),
            _ => (base_uv0, base_uv1, base_uv2, base_uv3),
        }
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

    fn append_quad_indices(
        base: u32,
        indices_u16: &mut Vec<u16>,
        indices_u32: &mut Option<Vec<u32>>,
    ) {
        let quad = [base, base + 1, base + 2, base, base + 2, base + 3];
        if let Some(indices) = indices_u32 {
            indices.extend_from_slice(&quad);
        } else if quad[5] <= u16::MAX as u32 {
            indices_u16.extend_from_slice(&[
                base as u16,
                (base + 1) as u16,
                (base + 2) as u16,
                base as u16,
                (base + 2) as u16,
                (base + 3) as u16,
            ]);
        } else {
            let mut indices = Vec::with_capacity(indices_u16.len() + quad.len());
            indices.extend(indices_u16.drain(..).map(u32::from));
            indices.extend_from_slice(&quad);
            *indices_u32 = Some(indices);
        }
    }

    /// Build complete mesh for a chunk.
    /// Reads blocks in storage order and emits visible faces without revisiting
    /// the chunk once for each face direction.
    pub fn build_chunk_mesh(world: &WorldData, coord: IVec3) -> Mesh {
        let mut positions = Vec::<[f32; 3]>::new();
        let mut normals = Vec::<[f32; 3]>::new();
        let mut uvs = Vec::<[f32; 2]>::new();
        let mut indices_u16 = Vec::<u16>::new();
        let mut indices_u32 = None;
        let Some(chunk) = world.chunks.get(&coord) else {
            return empty_mesh();
        };

        // Chunk storage is X-contiguous, so this loop reads it sequentially.
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let local = IVec3::new(x, y, z);
                    let block = chunk.get(local);
                    if block == AIR {
                        continue;
                    }

                    let block_pos = coord * CHUNK_SIZE + local;
                    let variant = block_variant(block_pos.x, block_pos.y, block_pos.z);
                    if matches!(block, RED_FLOWER | YELLOW_FLOWER | TALL_GRASS) {
                        let origin = local.as_vec3();
                        let planes = [
                            (
                                [
                                    origin,
                                    origin + Vec3::new(1.0, 0.0, 1.0),
                                    origin + Vec3::new(1.0, 1.0, 1.0),
                                    origin + Vec3::Y,
                                ],
                                Vec3::new(-1.0, 0.0, 1.0).normalize(),
                            ),
                            (
                                [
                                    origin + Vec3::X,
                                    origin + Vec3::Z,
                                    origin + Vec3::new(0.0, 1.0, 1.0),
                                    origin + Vec3::new(1.0, 1.0, 0.0),
                                ],
                                Vec3::new(1.0, 0.0, 1.0).normalize(),
                            ),
                        ];
                        let (uv0, uv1, uv2, uv3) = block_uv(block, 2, variant);
                        for (corners, normal) in planes {
                            for reverse in [false, true] {
                                let ordered = if reverse {
                                    [corners[3], corners[2], corners[1], corners[0]]
                                } else {
                                    corners
                                };
                                let base = positions.len() as u32;
                                for (corner, uv) in ordered.iter().zip([uv0, uv1, uv2, uv3].iter())
                                {
                                    positions.push(corner.to_array());
                                    normals.push(normal.to_array());
                                    uvs.push(*uv);
                                }
                                append_quad_indices(base, &mut indices_u16, &mut indices_u32);
                            }
                        }
                        continue;
                    }
                    for face_index in 0..6 {
                        let axis = face_index / 2;
                        let positive = face_index % 2 == 0;
                        let mut normal = IVec3::ZERO;
                        normal[axis] = if positive { 1 } else { -1 };

                        let crosses_chunk_boundary = if positive {
                            local[axis] == CHUNK_SIZE - 1
                        } else {
                            local[axis] == 0
                        };
                        let neighbor_block = if crosses_chunk_boundary {
                            super::block_at(world, block_pos + normal)
                        } else {
                            chunk.get(local + normal)
                        };
                        // Fluid blocks must not hide the solid face behind them.
                        if neighbor_block != AIR
                            && neighbor_block != WATER
                            && neighbor_block != LAVA
                        {
                            continue;
                        }

                        let u_axis = (axis + 1) % 3;
                        let v_axis = (axis + 2) % 3;
                        let mut origin = local;
                        origin[axis] += if positive { 1 } else { 0 };

                        let mut u_direction = IVec3::ZERO;
                        u_direction[u_axis] = 1;
                        let mut v_direction = IVec3::ZERO;
                        v_direction[v_axis] = 1;

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

                        let (uv0, uv1, uv2, uv3) = block_uv(block, face_index, variant);
                        let _shade = face_shade(face_index);
                        let base = positions.len() as u32;
                        for (corner, uv) in corners.iter().zip([uv0, uv1, uv2, uv3].iter()) {
                            positions.push(corner.to_array());
                            normals.push(normal.as_vec3().to_array());
                            uvs.push(*uv);
                        }
                        append_quad_indices(base, &mut indices_u16, &mut indices_u32);
                    }
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
        mesh.insert_indices(match indices_u32 {
            Some(indices) => Indices::U32(indices),
            None => Indices::U16(indices_u16),
        });
        mesh
    }
}

fn run_performance_bench() {
    const WARMUP_SAMPLES: usize = 8;
    const MEASURED_SAMPLES: usize = 128;
    const BENCH_SEED: u64 = 0xC0BE_2026;

    let mut generation_samples = Vec::with_capacity(MEASURED_SAMPLES);
    for sample in 0..WARMUP_SAMPLES + MEASURED_SAMPLES {
        let offset = sample as i32;
        let coord = IVec3::new(offset % 9 - 4, 4, offset / 9 - 2);
        let started = std::time::Instant::now();
        let chunk = generate_chunk(coord, BENCH_SEED);
        let elapsed = started.elapsed();
        std::hint::black_box(chunk);
        if sample >= WARMUP_SAMPLES {
            generation_samples.push(elapsed);
        }
    }

    let mut world = WorldData::default();
    for y in 3..=5 {
        for z in -1..=1 {
            for x in -1..=1 {
                let coord = IVec3::new(x, y, z);
                world
                    .chunks
                    .insert(coord, generate_chunk(coord, BENCH_SEED));
            }
        }
    }

    let mut mesh_coords = world.chunks.keys().copied().collect::<Vec<_>>();
    mesh_coords.sort_unstable_by_key(|coord| coord.to_array());
    let mut mesh_samples = Vec::with_capacity(MEASURED_SAMPLES);
    let mut mesh_bytes_total = 0usize;
    for sample in 0..WARMUP_SAMPLES + MEASURED_SAMPLES {
        let coord = mesh_coords[sample % mesh_coords.len()];
        let started = std::time::Instant::now();
        let mesh = rendering::build_chunk_mesh(&world, coord);
        let elapsed = started.elapsed();
        if sample >= WARMUP_SAMPLES {
            mesh_samples.push(elapsed);
            mesh_bytes_total += mesh_vertex_count(&mesh) * 32 + mesh_index_bytes(&mesh);
        }
        std::hint::black_box(mesh);
    }

    println!("The Cubes performance sample ({MEASURED_SAMPLES} measured iterations)");
    report_performance_samples("Chunk generation", generation_samples);
    report_performance_samples("Chunk mesh", mesh_samples);
    let mean_mesh_bytes = mesh_bytes_total / MEASURED_SAMPLES;
    let target_chunks = (STREAM_LOAD_RADIUS * 2 + 1).pow(2) as usize * 3;
    let resident_chunk_bound = (STREAM_UNLOAD_RADIUS * 2 + 1).pow(2) as usize * 3;
    println!("Sample mean chunk mesh buffers: {mean_mesh_bytes} bytes");
    println!(
        "Target chunks at load radius {STREAM_LOAD_RADIUS}: {target_chunks}; resident upper bound at unload radius {STREAM_UNLOAD_RADIUS}: {resident_chunk_bound}"
    );
    println!(
        "Estimated vertex/index payload at resident bound: {} MiB",
        mean_mesh_bytes * resident_chunk_bound / (1024 * 1024)
    );
    println!(
        "Chunk block storage at resident bound: {} MiB",
        resident_chunk_bound * CHUNK_VOLUME / (1024 * 1024)
    );
}

fn report_performance_samples(label: &str, mut samples: Vec<std::time::Duration>) {
    samples.sort_unstable();
    let total_nanos: u128 = samples.iter().map(std::time::Duration::as_nanos).sum();
    let average_ms = total_nanos as f64 / samples.len() as f64 / 1_000_000.0;
    let median_ms = samples[samples.len() / 2].as_secs_f64() * 1_000.0;
    let p95_index = ((samples.len() * 95 + 99) / 100).saturating_sub(1);
    let p95_ms = samples[p95_index].as_secs_f64() * 1_000.0;
    let max_ms = samples.last().unwrap().as_secs_f64() * 1_000.0;
    println!("{label}: mean {average_ms:.3} ms, median {median_ms:.3} ms, p95 {p95_ms:.3} ms, max {max_ms:.3} ms");
}

fn mesh_vertex_count(mesh: &Mesh) -> usize {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) => positions.len(),
        _ => 0,
    }
}

fn mesh_index_bytes(mesh: &Mesh) -> usize {
    match mesh.indices() {
        Some(Indices::U16(indices)) => indices.len() * 2,
        Some(Indices::U32(indices)) => indices.len() * 4,
        None => 0,
    }
}

fn run_plains_generation_report() {
    let seeds = [42, 1337, 812, 2026, 987_654_321];
    println!("Pradera: resumen determinista para varias semillas");
    for seed in seeds {
        let (spawn_x, spawn_y, spawn_z) = world_spawn_point(seed, TERRAIN_VERSION);
        println!(
            "seed {seed}: aparición seca en ({:.0}, {}, {:.0})",
            spawn_x, spawn_y, spawn_z
        );
        let mut plains_columns = 0usize;
        let mut columns = 0usize;
        let mut tall_grass = 0usize;
        let mut red_flowers = 0usize;
        let mut yellow_flowers = 0usize;
        let mut wheat = 0usize;
        for sample_z in -6..=6 {
            for sample_x in -6..=6 {
                // Sample across a broad climate range while generating only
                // one chunk per point, so this stays fast enough for tuning.
                let chunk_x = sample_x * 8;
                let chunk_z = sample_z * 8;
                let center_x = chunk_x * CHUNK_SIZE + CHUNK_SIZE / 2;
                let center_z = chunk_z * CHUNK_SIZE + CHUNK_SIZE / 2;
                let surface_y = terrain_height(center_x, center_z, seed);
                let chunk = generate_chunk(
                    IVec3::new(chunk_x, surface_y.div_euclid(CHUNK_SIZE), chunk_z),
                    seed,
                );
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        columns += 1;
                        if world::terrain::biome_at(
                            chunk_x * CHUNK_SIZE + x,
                            chunk_z * CHUNK_SIZE + z,
                            seed,
                        ) == Biome::Plains
                        {
                            plains_columns += 1;
                        }
                    }
                }
                tall_grass += chunk
                    .blocks
                    .iter()
                    .filter(|&&block| block == TALL_GRASS)
                    .count();
                red_flowers += chunk
                    .blocks
                    .iter()
                    .filter(|&&block| block == RED_FLOWER)
                    .count();
                yellow_flowers += chunk
                    .blocks
                    .iter()
                    .filter(|&&block| block == YELLOW_FLOWER)
                    .count();
                wheat += chunk.blocks.iter().filter(|&&block| block == WHEAT).count();
            }
        }
        println!(
            "seed {seed}: {plains_columns}/{columns} columnas de pradera; hierba alta {tall_grass}, flores rojas {red_flowers}, amarillas {yellow_flowers}, trigo {wheat}"
        );
    }
}

fn main() {
    if std::env::args().any(|argument| argument == "--perf-bench") {
        run_performance_bench();
        return;
    }
    if std::env::args().any(|argument| argument == "--plains-report") {
        run_plains_generation_report();
        return;
    }
    if std::env::args().any(|argument| argument == "--world-list-report") {
        match data_directory()
            .and_then(|directory| fs::read_dir(directory).map_err(|error| format!("{error}")))
        {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|extension| extension.to_str()) != Some("ron") {
                        continue;
                    }
                    match load_world(&path) {
                        Ok(save) => println!("OK: {} ({})", save.name, path.display()),
                        Err(error) => println!("ERROR: {}: {error}", path.display()),
                    }
                }
            }
            Err(error) => eprintln!("No se pudo listar los mundos: {error}"),
        }
        return;
    }

    App::new()
        .insert_resource(Time::<Fixed>::from_hz(120.0))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "The Cubes".into(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
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
        .init_resource::<MiningState>()
        .init_resource::<MiningVisuals>()
        .init_resource::<CraftingUi>()
        .init_resource::<ChatState>()
        .init_resource::<WorldForm>()
        .init_resource::<PendingWorld>()
        .init_resource::<SelectedWorld>()
        .init_resource::<TextureAssets>()
        .init_resource::<WorldStorage>()
        .init_resource::<ChunkStreamingState>()
        .init_resource::<LoadingProgress>()
        .init_resource::<DayNightCycle>()
        .init_resource::<MobPopulation>()
        .init_resource::<MobAssets>()
        .init_state::<AppState>()
        .add_plugins(WorldScenePlugin)
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
        .add_systems(OnEnter(AppState::WorldSelect), setup_world_select)
        .add_systems(OnExit(AppState::WorldSelect), cleanup_menu)
        .add_systems(
            OnEnter(AppState::ConfirmDeleteWorld),
            setup_delete_world_confirmation,
        )
        .add_systems(OnExit(AppState::ConfirmDeleteWorld), cleanup_menu)
        .add_systems(OnEnter(AppState::CreateWorld), setup_create_world)
        .add_systems(OnExit(AppState::CreateWorld), cleanup_menu)
        .add_systems(OnEnter(AppState::LoadingWorld), begin_loading_world)
        .add_systems(OnEnter(AppState::LoadingWorld), setup_loading_screen)
        .add_systems(OnExit(AppState::LoadingWorld), cleanup_menu)
        .add_systems(OnEnter(AppState::Paused), setup_pause_menu)
        .add_systems(OnExit(AppState::Paused), cleanup_pause_menu)
        .add_systems(OnExit(AppState::InGame), reset_mining)
        .add_systems(OnExit(AppState::InGame), cleanup_plains_ambience)
        .add_systems(OnEnter(AppState::MainMenu), cleanup_game_scene)
        .add_systems(FixedUpdate, move_player.run_if(in_state(AppState::InGame)))
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
                        .or(in_state(AppState::ConfirmDeleteWorld))
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
                    chat_input,
                    update_chat_bar,
                    look,
                    update_survival,
                    consume_stored_food,
                    advance_day_night,
                    update_day_night_lighting,
                    update_sky,
                    stream_world_chunks,
                    update_particles,
                    animate_water_lava,
                    update_plains_ambience,
                ),
                update_mobs,
                (
                    select_block.before(sync_selected_block),
                    select_hotbar.before(sync_selected_block),
                    (
                        inventory_controls,
                        inventory_clicks,
                        sync_selected_block,
                        update_inventory_ui,
                    )
                        .chain()
                        .before(edit_blocks)
                        .before(pause_from_game),
                    update_survival_hud,
                    update_hotbar,
                    update_block_outline,
                    edit_blocks,
                    update_block_drops.after(edit_blocks),
                    update_mining_visual.after(edit_blocks),
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

struct WorldScenePlugin;

impl Plugin for WorldScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::LoadingWorld), load_textures)
            .add_systems(OnEnter(AppState::InGame), setup_game_scene);
    }
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    MainMenu,
    WorldSelect,
    ConfirmDeleteWorld,
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
struct ChatBar;

#[derive(Component)]
struct ChatInputText;

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
    PlaySelected,
    Create,
    Back,
    DeleteSelected,
    ConfirmDelete,
    CancelDelete,
    StartWorld,
    Resume,
    SaveAndMenu,
    CancelLoading,
    Exit,
}

#[derive(Component)]
struct SelectWorldAction {
    path: PathBuf,
    name: String,
}

#[derive(Component)]
struct WorldSelectionStatus;

#[derive(Component)]
struct BlockParticle {
    lifetime: f32,
    max_lifetime: f32,
    velocity: Vec3,
}

#[derive(Component)]
struct PlainsAmbience;

#[derive(Component)]
struct BlockDrop {
    item: Item,
    velocity: Vec3,
    age: f32,
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
            seed: new_world_seed().to_string(),
            focus: WorldField::Name,
            error: None,
        }
    }
}

fn new_world_seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    timestamp
        ^ (u64::from(std::process::id()).rotate_left(23))
        ^ SEQUENCE
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[derive(Resource, Default)]
struct PendingWorld {
    save: Option<WorldSave>,
    path: Option<PathBuf>,
    is_new_world: bool,
}

#[derive(Resource, Default)]
struct SelectedWorld {
    path: Option<PathBuf>,
    name: Option<String>,
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
    #[serde(default = "legacy_terrain_version")]
    terrain_version: u32,
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
    #[serde(default, deserialize_with = "deserialize_inventory_counts")]
    extra_inventory: Vec<u32>,
    #[serde(default)]
    slot_inventory: Option<Inventory>,
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

#[derive(Resource)]
struct WorldData {
    chunks: HashMap<IVec3, Chunk>,
    modified_chunks: HashMap<IVec3, Chunk>,
    dirty: HashSet<IVec3>,
    name: String,
    seed: u64,
    terrain_version: u32,
    door_states: HashMap<IVec3, DoorState>,
}

fn legacy_terrain_version() -> u32 {
    1
}

fn deserialize_inventory_counts<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CountsVisitor;

    impl<'de> serde::de::Visitor<'de> for CountsVisitor {
        type Value = Vec<u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("una secuencia de cantidades de objetos")
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut counts = Vec::new();
            while let Some(count) = sequence.next_element()? {
                counts.push(count);
            }
            Ok(counts)
        }
    }

    deserializer.deserialize_any(CountsVisitor)
}

impl Default for WorldData {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            modified_chunks: HashMap::new(),
            dirty: HashSet::new(),
            name: String::new(),
            seed: 0,
            terrain_version: TERRAIN_VERSION,
            door_states: HashMap::new(),
        }
    }
}

#[derive(Resource, Default)]
struct WorldStorage {
    path: Option<PathBuf>,
    legacy_chunks: HashMap<IVec3, Chunk>,
    dirty_chunks: HashSet<IVec3>,
    failed_chunks: HashSet<IVec3>,
    autosave_elapsed: f32,
    retry_elapsed: f32,
    retry_generation: u64,
    metadata_save_requested: bool,
    status: String,
}

#[derive(Resource, Default)]
struct ChunkStreamingState {
    center: Option<IVec3>,
    vertical_radius: i32,
    retry_generation: u64,
    pending_unloads: Vec<(Entity, IVec3)>,
    next_unload: usize,
    pending: Vec<IVec3>,
    next: usize,
    chunk_entities: HashMap<IVec3, Entity>,
}

impl ChunkStreamingState {
    fn invalidate(&mut self) {
        self.center = None;
        self.pending_unloads.clear();
        self.next_unload = 0;
        self.pending.clear();
        self.next = 0;
        self.chunk_entities.clear();
    }
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
    jump_buffer: f32,
    coyote_time: f32,
    step_up_remaining: f32,
    jump_key_down: bool,
    flight_key_down: bool,
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
struct ExperienceText;

#[derive(Component, Clone, Copy)]
struct SurvivalIcon {
    kind: SurvivalIconKind,
    index: usize,
}

#[derive(Clone, Copy)]
enum SurvivalIconKind {
    Health,
    Hunger,
}

#[derive(Component)]
struct ExperienceFill;

#[derive(Component)]
struct InventoryPanel;

#[derive(Component, Clone, Copy)]
enum InventoryTarget {
    Slot(usize),
    Small(usize),
    Large(usize),
    SmallResult,
    LargeResult,
}

#[derive(Component)]
struct InventoryLabel(InventoryTarget);

#[derive(Component)]
struct InventoryCursorLabel;

#[derive(Component)]
struct InventoryGridMode(bool);

#[derive(Component)]
struct HotbarSlotVisual {
    slot_index: usize,
}

#[derive(Component)]
struct SurvivalHeartsDisplay;

#[derive(Component)]
struct SurvivalHungerDisplay;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FoodType {
    BerryRation, // saturation: 4
    Apple,       // saturation: 6
    Mushroom,    // saturation: 2 (puede ser venenoso)
    Wheat,       // saturation: 8
    RawMeat,     // saturation: 3
    CookedMeat,  // saturation: 8
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
}

#[derive(Resource, Default)]
struct ChatState {
    open: bool,
    input: String,
}

#[derive(Resource, Default)]
struct CraftedTools {
    pickaxe_durability: u32,
    axe_durability: u32,
    shovel_durability: u32,
    sword_durability: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MiningTool {
    Pickaxe,
    Axe,
    Shovel,
}

#[derive(Clone, Copy)]
struct MiningTarget {
    position: IVec3,
    block: u8,
    tool: Option<MiningTool>,
    elapsed: f32,
    duration: f32,
}

#[derive(Resource, Default)]
struct MiningState {
    target: Option<MiningTarget>,
}

#[derive(Resource, Default)]
struct MiningVisuals {
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
struct MiningOverlay;

#[derive(Component)]
struct BlockOutline;

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
struct SunLighting;

#[derive(Component)]
struct MoonLighting;

#[derive(Component)]
struct HotbarSlot(usize);

#[derive(Component)]
struct HotbarCount(usize);

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

fn generate_legacy_chunk(coord: IVec3, seed: u64) -> Chunk {
    let mut chunk = Chunk::new();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let height = legacy_terrain_height(world_x, world_z, seed);
            let biome = legacy_biome_at(world_x, world_z, seed);
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
            let height = legacy_terrain_height(world_x, world_z, seed);
            if legacy_biome_at(world_x, world_z, seed) != Biome::Forest
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
                                if coordinate_hash(world_x ^ leaf_x, leaf_y, seed ^ 0x5C0F) % 100
                                    < 15
                                {
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
            if legacy_biome_at(world_x, world_z, seed) != Biome::Forest {
                continue;
            }
            let height = legacy_terrain_height(world_x, world_z, seed);
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
            if legacy_biome_at(world_x, world_z, seed) != Biome::Plains {
                continue;
            }
            let height = legacy_terrain_height(world_x, world_z, seed);
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

fn generate_chunk(coord: IVec3, seed: u64) -> Chunk {
    let generated = world::terrain::generate_chunk_with_palette(
        coord,
        seed,
        world::terrain::TerrainBlockPalette {
            air: AIR,
            grass: GRASS,
            dirt: DIRT,
            stone: STONE,
            sand: SAND,
            wood: WOOD,
            leaves: LEAVES,
            apple: APPLE,
            wheat: WHEAT,
            mushroom: MUSHROOM,
            water: WATER,
            red_flower: RED_FLOWER,
            yellow_flower: YELLOW_FLOWER,
            tall_grass: TALL_GRASS,
            planks: PLANKS,
        },
    );
    Chunk {
        blocks: generated.blocks,
    }
}

fn generate_v2_chunk(coord: IVec3, seed: u64) -> Chunk {
    const COLUMN_SIDE: usize = (CHUNK_SIZE + 4) as usize;
    const COLUMN_COUNT: usize = COLUMN_SIDE * COLUMN_SIDE;

    let mut chunk = Chunk::new();
    let base_y = coord.y * CHUNK_SIZE;
    let mut surface_columns = [(0, Biome::Plains); COLUMN_COUNT];

    for z in -2..CHUNK_SIZE + 2 {
        for x in -2..CHUNK_SIZE + 2 {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let height = terrain_height_v2(world_x, world_z, seed);
            let biome = biome_at_with_height_v2(world_x, world_z, seed, height);
            let index = ((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize;
            surface_columns[index] = (height, biome);
        }
    }

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let column_index = ((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize;
            let (height, biome) = surface_columns[column_index];

            for local_y in 0..CHUNK_SIZE {
                let world_y = base_y + local_y;
                if !(0..WORLD_HEIGHT).contains(&world_y) {
                    continue;
                }

                if world_y <= height {
                    if world_y < height - 4 && is_cave_v2(world_x, world_y, world_z, seed) {
                        if world_y <= SEA_LEVEL && world_y > 5 {
                            chunk.set(IVec3::new(x, local_y, z), WATER);
                        }
                        continue;
                    }

                    let depth = height - world_y;
                    let block = if depth == 0 {
                        match biome {
                            Biome::Plains | Biome::Forest => GRASS,
                            Biome::Desert | Biome::Beach => SAND,
                            Biome::Mountain | Biome::Volcano | Biome::Cave => STONE,
                            Biome::Tundra | Biome::Swamp => STONE,
                        }
                    } else if biome == Biome::Desert && depth <= 5 {
                        SAND
                    } else if biome == Biome::Beach && depth <= 4 {
                        SAND
                    } else if biome == Biome::Mountain && depth > 3 {
                        STONE
                    } else if depth <= 4 {
                        DIRT
                    } else {
                        STONE
                    };
                    chunk.set(IVec3::new(x, local_y, z), block);
                } else if world_y <= SEA_LEVEL && height < SEA_LEVEL {
                    chunk.set(IVec3::new(x, local_y, z), WATER);
                }
            }
        }
    }

    for z in -2..CHUNK_SIZE + 2 {
        for x in -2..CHUNK_SIZE + 2 {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let column_index = ((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize;
            let (height, biome) = surface_columns[column_index];
            if biome != Biome::Forest
                || height <= SEA_LEVEL
                || coordinate_hash(world_x, world_z, seed ^ 0xA24B_AED4_963E_E407) % 100 >= 9
            {
                continue;
            }

            let tree_height =
                4 + (coordinate_hash(world_z, world_x, seed ^ 0xA24B_AED4_963E_E407) % 3) as i32;
            for trunk_y in height + 1..=height + tree_height {
                set_generated_block(&mut chunk, x, trunk_y - base_y, z, WOOD);
            }
            let crown_y = height + tree_height;
            for leaf_y in crown_y - 2..=crown_y + 1 {
                let radius: i32 = if leaf_y == crown_y + 1 { 1 } else { 2 };
                for leaf_z in -radius..=radius {
                    for leaf_x in -radius..=radius {
                        if leaf_y == crown_y + 1 && leaf_x.abs() + leaf_z.abs() > 1 {
                            continue;
                        }
                        let local_x = x + leaf_x;
                        let local_z = z + leaf_z;
                        let local_y = leaf_y - base_y;
                        if !(0..CHUNK_SIZE).contains(&local_x)
                            || !(0..CHUNK_SIZE).contains(&local_z)
                            || !(0..CHUNK_SIZE).contains(&local_y)
                        {
                            continue;
                        }
                        let local = IVec3::new(local_x, local_y, local_z);
                        if chunk.get(local) == AIR {
                            let leaf = if coordinate_hash(
                                world_x ^ leaf_x,
                                world_z ^ leaf_z,
                                seed ^ 0x5C0F,
                            ) % 100
                                < 4
                            {
                                APPLE
                            } else {
                                LEAVES
                            };
                            chunk.set(local, leaf);
                        }
                    }
                }
            }
        }
    }

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let column_index = ((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize;
            let (height, biome) = surface_columns[column_index];
            let local_surface_y = height - base_y;
            let feature = coordinate_hash(world_x, world_z, seed ^ 0xDEAD_BEEF) % 100;
            let vegetation = match biome {
                Biome::Forest if feature < 4 => MUSHROOM,
                Biome::Plains if feature < 5 => WHEAT,
                _ => AIR,
            };
            if vegetation != AIR {
                set_generated_block(&mut chunk, x, local_surface_y + 1, z, vegetation);
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
    mut texture_assets: ResMut<TextureAssets>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if texture_assets.atlas.is_none() {
        let mut atlas = Image::new(
            Extent3d {
                width: texture_layout::ATLAS_SIZE,
                height: texture_layout::ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            include_bytes!(concat!(env!("OUT_DIR"), "/block_atlas.rgba")).to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        atlas.sampler = ImageSampler::nearest();
        texture_assets.atlas = Some(images.add(atlas));

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

fn create_mining_materials(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let segments: [(i32, i32, i32, i32); 16] = [
        (16, 16, 14, 10),
        (14, 10, 11, 5),
        (16, 16, 21, 12),
        (21, 12, 27, 11),
        (16, 16, 20, 20),
        (20, 20, 22, 27),
        (16, 16, 11, 19),
        (11, 19, 5, 21),
        (14, 10, 17, 7),
        (11, 5, 8, 2),
        (21, 12, 23, 6),
        (27, 11, 30, 8),
        (20, 20, 26, 22),
        (22, 27, 19, 30),
        (11, 19, 10, 25),
        (5, 21, 2, 18),
    ];
    (0..MINING_CRACK_STAGES)
        .map(|stage| {
            let mut pixels = vec![0_u8; 32 * 32 * 4];
            for &(mut x, mut y, end_x, end_y) in &segments[..(stage + 1) * 2] {
                let dx: i32 = (end_x - x).abs();
                let dy: i32 = -(end_y - y).abs();
                let step_x = if x < end_x { 1 } else { -1 };
                let step_y = if y < end_y { 1 } else { -1 };
                let mut error = dx + dy;
                loop {
                    pixels[((y * 32 + x) * 4 + 3) as usize] = 220;
                    if x == end_x && y == end_y {
                        break;
                    }
                    let twice_error = 2 * error;
                    if twice_error >= dy {
                        error += dy;
                        x += step_x;
                    }
                    if twice_error <= dx {
                        error += dx;
                        y += step_y;
                    }
                }
            }
            let mut image = Image::new(
                Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect()
}

fn update_mining_visual(
    mining: Res<MiningState>,
    visuals: Res<MiningVisuals>,
    mut overlay: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<MiningOverlay>,
    >,
) {
    let Ok((mut transform, mut visibility, mut material)) = overlay.get_single_mut() else {
        return;
    };
    if let Some(target) = mining.target {
        let stage = ((target.elapsed / target.duration) * MINING_CRACK_STAGES as f32)
            .floor()
            .clamp(0.0, (MINING_CRACK_STAGES - 1) as f32) as usize;
        if let Some(handle) = visuals.materials.get(stage) {
            transform.translation = target.position.as_vec3() + Vec3::splat(0.5);
            material.0 = handle.clone();
            *visibility = Visibility::Visible;
        } else {
            error!("Mining overlay material for stage {stage} is missing");
            *visibility = Visibility::Hidden;
        }
    } else {
        *visibility = Visibility::Hidden;
    }
}

fn update_block_outline(
    world: Res<WorldData>,
    ui: Res<CraftingUi>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<&Transform, (With<Player>, Without<BlockOutline>)>,
    mut outline: Query<(&mut Transform, &mut Visibility), (With<BlockOutline>, Without<Player>)>,
) {
    let Ok((mut outline_transform, mut visibility)) = outline.get_single_mut() else {
        return;
    };
    if ui.open || ui.inventory_open {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok(window) = windows.get_single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok(camera) = camera.get_single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        *visibility = Visibility::Hidden;
        return;
    }
    let direction = camera.rotation * Vec3::NEG_Z;
    if let Some((target, _, _)) =
        raycast_with_distance(&world, camera.translation, direction, REACH)
    {
        outline_transform.translation = target.as_vec3() + Vec3::splat(0.5);
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}

fn reset_mining(
    mut mining: ResMut<MiningState>,
    mut overlays: Query<&mut Visibility, With<MiningOverlay>>,
    mut outlines: Query<&mut Visibility, With<BlockOutline>>,
) {
    mining.target = None;
    for mut visibility in &mut overlays {
        *visibility = Visibility::Hidden;
    }
    for mut visibility in &mut outlines {
        *visibility = Visibility::Hidden;
    }
}

fn setup_game_scene(
    mut commands: Commands,
    pending: Res<PendingWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mob_assets: ResMut<MobAssets>,
    mut mining_visuals: ResMut<MiningVisuals>,
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
    if mining_visuals.materials.is_empty() {
        mining_visuals.materials = create_mining_materials(&mut images, &mut materials);
    }
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.01, 1.01, 1.01))),
        MeshMaterial3d(mining_visuals.materials[0].clone()),
        Transform::default(),
        Visibility::Hidden,
        MiningOverlay,
        GameEntity,
    ));

    let outline_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let outline_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.015, 0.015, 0.015, 0.9),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let edge_thickness = 0.025;
    let edge_offset = 0.51;
    let edges = [
        (
            Vec3::new(1.04, edge_thickness, edge_thickness),
            Vec3::new(0.0, -edge_offset, -edge_offset),
        ),
        (
            Vec3::new(1.04, edge_thickness, edge_thickness),
            Vec3::new(0.0, -edge_offset, edge_offset),
        ),
        (
            Vec3::new(1.04, edge_thickness, edge_thickness),
            Vec3::new(0.0, edge_offset, -edge_offset),
        ),
        (
            Vec3::new(1.04, edge_thickness, edge_thickness),
            Vec3::new(0.0, edge_offset, edge_offset),
        ),
        (
            Vec3::new(edge_thickness, 1.04, edge_thickness),
            Vec3::new(-edge_offset, 0.0, -edge_offset),
        ),
        (
            Vec3::new(edge_thickness, 1.04, edge_thickness),
            Vec3::new(-edge_offset, 0.0, edge_offset),
        ),
        (
            Vec3::new(edge_thickness, 1.04, edge_thickness),
            Vec3::new(edge_offset, 0.0, -edge_offset),
        ),
        (
            Vec3::new(edge_thickness, 1.04, edge_thickness),
            Vec3::new(edge_offset, 0.0, edge_offset),
        ),
        (
            Vec3::new(edge_thickness, edge_thickness, 1.04),
            Vec3::new(-edge_offset, -edge_offset, 0.0),
        ),
        (
            Vec3::new(edge_thickness, edge_thickness, 1.04),
            Vec3::new(-edge_offset, edge_offset, 0.0),
        ),
        (
            Vec3::new(edge_thickness, edge_thickness, 1.04),
            Vec3::new(edge_offset, -edge_offset, 0.0),
        ),
        (
            Vec3::new(edge_thickness, edge_thickness, 1.04),
            Vec3::new(edge_offset, edge_offset, 0.0),
        ),
    ];
    commands
        .spawn((
            Transform::default(),
            Visibility::Hidden,
            BlockOutline,
            GameEntity,
        ))
        .with_children(|children| {
            for (scale, translation) in edges {
                children.spawn((
                    Mesh3d(outline_mesh.clone()),
                    MeshMaterial3d(outline_material.clone()),
                    Transform::from_translation(translation).with_scale(scale),
                ));
            }
        });

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
            jump_buffer: 0.0,
            coyote_time: 0.0,
            step_up_remaining: 0.0,
            jump_key_down: false,
            flight_key_down: false,
        },
    ));

    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 3_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.0)),
        SunLighting,
        GameEntity,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 0.0,
            color: Color::srgb(0.48, 0.62, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::default(),
        MoonLighting,
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
    setup_slot_inventory_ui(&mut commands);
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
    mut sun: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<SunLighting>, Without<MoonLighting>),
    >,
    mut moon: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<MoonLighting>, Without<SunLighting>),
    >,
    mut ambient: ResMut<AmbientLight>,
) {
    let (elevation, daylight) = sun_position_and_daylight(cycle.time_of_day_seconds);
    let sunlight = if elevation > 0.0 {
        smoothstep(0.0, 0.15, elevation)
    } else {
        0.0
    };
    let sun_direction = sun_direction(cycle.time_of_day_seconds);

    if let Ok((mut transform, mut light)) = sun.get_single_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction);
        light.illuminance = sunlight * 3_000.0;
        light.color = Color::srgb(1.0, 0.62 + daylight * 0.38, 0.42 + daylight * 0.58);
    }

    if let Ok((mut transform, mut light)) = moon.get_single_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sun_direction);
        light.illuminance = (1.0 - sunlight) * 160.0;
        light.color = Color::srgb(0.48, 0.62, 1.0);
    }

    ambient.color = Color::srgb(
        0.62 + sunlight * 0.16,
        0.65 + sunlight * 0.15,
        0.70 + sunlight * 0.12,
    );
    ambient.brightness = 350.0 + sunlight * 700.0;
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

fn setup_world_select(mut commands: Commands, mut selected: ResMut<SelectedWorld>) {
    let worlds = match list_saved_worlds() {
        Ok(worlds) => worlds,
        Err(error) => {
            error!("Could not list saved worlds: {error}");
            Vec::new()
        }
    };

    if selected
        .path
        .as_ref()
        .is_some_and(|path| !worlds.iter().any(|(_, world_path)| world_path == path))
    {
        *selected = SelectedWorld::default();
    }

    let selected_label = selected
        .name
        .as_ref()
        .map(|name| format!("Seleccionado: {name}"))
        .unwrap_or_else(|| "Selecciona un mundo".to_string());

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
                let is_selected = selected.path.as_ref() == Some(&path);
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(360.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(if is_selected {
                        Color::srgb(0.18, 0.36, 0.29)
                    } else {
                        Color::srgb(0.12, 0.2, 0.25)
                    }),
                    SelectWorldAction {
                        path,
                        name: name.clone(),
                    },
                ))
                .with_child((
                    Text::new(name),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
            root.spawn((
                Text::new(selected_label),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.72, 0.86, 0.8)),
                WorldSelectionStatus,
            ));
            for (label, action) in [
                ("Jugar al mundo seleccionado", MenuCommand::PlaySelected),
                ("Crear nuevo mundo", MenuCommand::Create),
                ("Eliminar mundo", MenuCommand::DeleteSelected),
                ("Cancelar", MenuCommand::Back),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(360.0),
                        height: Val::Px(44.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

fn setup_delete_world_confirmation(mut commands: Commands, selected: Res<SelectedWorld>) {
    let name = selected.name.as_deref().unwrap_or("mundo seleccionado");
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
                Text::new(format!("¿Eliminar el mundo ‘{name}’ permanentemente?")),
                TextFont::from_font_size(24.0),
                TextColor(Color::WHITE),
            ));
            for (label, action) in [
                ("Eliminar", MenuCommand::ConfirmDelete),
                ("Cancelar", MenuCommand::CancelDelete),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(48.0),
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
    mut chat: ResMut<ChatState>,
    mut pending: ResMut<PendingWorld>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    mut storage: ResMut<WorldStorage>,
    mut streaming: ResMut<ChunkStreamingState>,
    mut mob_population: ResMut<MobPopulation>,
) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
    world.chunks.clear();
    world.modified_chunks.clear();
    world.dirty.clear();
    world.name.clear();
    *inventory = Inventory::default();
    *survival = Survival::default();
    *crafted_tools = CraftedTools::default();
    *crafting_ui = CraftingUi::default();
    *chat = ChatState::default();
    selected.0 = GRASS;
    selected_slot.0 = 0;
    pending.save = None;
    pending.path = None;
    pending.is_new_world = false;
    *storage = WorldStorage::default();
    streaming.invalidate();
    *mob_population = MobPopulation::default();
}

fn block_type_name(block_type: u8) -> &'static str {
    block_definition(block_type).name_es
}

fn item_name(item: Item) -> &'static str {
    match item {
        Item::Block(block) => block_type_name(block),
        _ => item.definition().name,
    }
}

fn spawn_inventory_slot(parent: &mut ChildBuilder, target: InventoryTarget) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(72.0),
                height: Val::Px(52.0),
                border: UiRect::all(Val::Px(1.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.17, 0.19)),
            BorderColor(Color::srgb(0.55, 0.55, 0.55)),
            target,
        ))
        .with_child((
            Text::new(""),
            TextFont::from_font_size(12.0),
            TextColor(Color::WHITE),
            InventoryLabel(target),
        ));
}

fn setup_slot_inventory_ui(commands: &mut Commands) {
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(100),
            InventoryPanel,
            GameEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(750.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.12, 0.15)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Inventario [E] | Mesa 3x3 [C] | Cerrar [Esc]"),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Izquierdo: mover / combinar | Derecho: dividir / colocar uno"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Recetas: consulta la guia en README.md"),
                    TextFont::from_font_size(13.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Cursor: vacio"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                    InventoryCursorLabel,
                ));
                for (large, width) in [(false, 2), (true, 3)] {
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(12.0),
                                ..default()
                            },
                            InventoryGridMode(large),
                        ))
                        .with_children(|row| {
                            row.spawn(Node {
                                width: Val::Px(width as f32 * 76.0),
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(4.0),
                                row_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|grid| {
                                for index in 0..width * width {
                                    spawn_inventory_slot(
                                        grid,
                                        if large {
                                            InventoryTarget::Large(index)
                                        } else {
                                            InventoryTarget::Small(index)
                                        },
                                    );
                                }
                            });
                            row.spawn((
                                Text::new("->"),
                                TextFont::from_font_size(24.0),
                                TextColor(Color::WHITE),
                            ));
                            spawn_inventory_slot(
                                row,
                                if large {
                                    InventoryTarget::LargeResult
                                } else {
                                    InventoryTarget::SmallResult
                                },
                            );
                        });
                }
                panel.spawn((
                    Text::new("Espacios 1-9: barra rapida"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                ));
                panel
                    .spawn(Node {
                        width: Val::Px(684.0),
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(4.0),
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|grid| {
                        for index in 0..INVENTORY_SIZE {
                            spawn_inventory_slot(grid, InventoryTarget::Slot(index));
                        }
                    });
            });
        });
}

fn inventory_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<CraftingUi>,
    chat: Res<ChatState>,
    mut panel: Query<&mut Node, With<InventoryPanel>>,
    mut modes: Query<(&InventoryGridMode, &mut Node), Without<InventoryPanel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    ui.just_closed = false;
    if chat.open {
        return;
    }
    let active = ui.open || ui.inventory_open;
    let mode = if keys.just_pressed(KeyCode::Escape) && active {
        None
    } else if keys.just_pressed(KeyCode::KeyE) {
        if ui.inventory_open {
            None
        } else {
            Some(false)
        }
    } else if keys.just_pressed(KeyCode::KeyC) {
        if ui.open {
            None
        } else {
            Some(true)
        }
    } else {
        return;
    };
    let Ok(mut window) = windows.get_single_mut() else {
        error!("Primary window is missing while toggling inventory");
        return;
    };
    let Ok(mut root) = panel.get_single_mut() else {
        error!("Inventory panel is missing");
        return;
    };
    ui.open = mode == Some(true);
    ui.inventory_open = mode == Some(false);
    ui.just_closed = active && mode.is_none();
    root.display = if mode.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for (grid, mut node) in &mut modes {
        node.display = if mode == Some(grid.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    window.cursor_options.grab_mode = if mode.is_some() {
        CursorGrabMode::None
    } else {
        CursorGrabMode::Locked
    };
    window.cursor_options.visible = mode.is_some();
}

fn inventory_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    ui: Res<CraftingUi>,
    mut inventory: ResMut<Inventory>,
    buttons: Query<(&Interaction, &InventoryTarget)>,
) {
    if !ui.open && !ui.inventory_open {
        return;
    }
    let right = mouse.just_pressed(MouseButton::Right);
    if !right && !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((_, target)) = buttons.iter().find(|(interaction, target)| {
        **interaction != Interaction::None
            && match target {
                InventoryTarget::Small(_) | InventoryTarget::SmallResult => ui.inventory_open,
                InventoryTarget::Large(_) | InventoryTarget::LargeResult => ui.open,
                InventoryTarget::Slot(_) => true,
            }
    }) else {
        return;
    };
    let inventory = &mut *inventory;
    match *target {
        InventoryTarget::Slot(index) => {
            item_inventory::click_slot(&mut inventory.slots[index], &mut inventory.cursor, right)
        }
        InventoryTarget::Small(index) => item_inventory::click_slot(
            &mut inventory.small_grid[index],
            &mut inventory.cursor,
            right,
        ),
        InventoryTarget::Large(index) => item_inventory::click_slot(
            &mut inventory.large_grid[index],
            &mut inventory.cursor,
            right,
        ),
        InventoryTarget::SmallResult => {
            item_inventory::craft(&mut inventory.small_grid, 2, &mut inventory.cursor);
        }
        InventoryTarget::LargeResult => {
            item_inventory::craft(&mut inventory.large_grid, 3, &mut inventory.cursor);
        }
    }
}

fn update_inventory_ui(
    inventory: Res<Inventory>,
    mut labels: Query<(&InventoryLabel, &mut Text), Without<InventoryCursorLabel>>,
    mut cursor: Query<&mut Text, With<InventoryCursorLabel>>,
) {
    if !inventory.is_changed() {
        return;
    }
    for (label, mut text) in &mut labels {
        let stack = match label.0 {
            InventoryTarget::Slot(index) => inventory.slots[index],
            InventoryTarget::Small(index) => inventory.small_grid[index],
            InventoryTarget::Large(index) => inventory.large_grid[index],
            InventoryTarget::SmallResult => item_inventory::recipe(&inventory.small_grid, 2),
            InventoryTarget::LargeResult => item_inventory::recipe(&inventory.large_grid, 3),
        };
        text.0 = stack.map_or_else(String::new, |stack| {
            format!("{}\nx{}", item_name(stack.item), stack.count)
        });
    }
    for mut text in &mut cursor {
        text.0 = inventory.cursor.map_or_else(
            || "Cursor: vacio".into(),
            |stack| format!("Cursor: {} x{}", item_name(stack.item), stack.count),
        );
    }
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
                    left: Val::Px(20.0),
                    bottom: Val::Px(154.0),
                    width: Val::Px(620.0),
                    min_height: Val::Px(42.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    align_items: AlignItems::Center,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.78)),
                GlobalZIndex(50),
                ChatBar,
            ))
            .with_child((
                Text::new(""),
                TextFont::from_font_size(18.0),
                TextColor(Color::WHITE),
                ChatInputText,
            ));

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(92.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|hud| {
                hud.spawn(Node {
                    width: Val::Px(548.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|center| {
                    center
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            for (kind, symbol, color) in
                                [(SurvivalIconKind::Health, "♥", Color::srgb(0.92, 0.16, 0.18))]
                            {
                                row.spawn(Node {
                                    width: Val::Px(218.0),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|icons| {
                                    for index in 0..10 {
                                        icons.spawn((
                                            Text::new(symbol),
                                            TextFont::from_font_size(21.0),
                                            TextColor(color),
                                            SurvivalIcon { kind, index },
                                        ));
                                    }
                                });
                            }

                            row.spawn(Node {
                                width: Val::Px(218.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|icons| {
                                for index in 0..10 {
                                    icons.spawn((
                                        Text::new("●"),
                                        TextFont::from_font_size(19.0),
                                        TextColor(Color::srgb(0.86, 0.58, 0.20)),
                                        SurvivalIcon {
                                            kind: SurvivalIconKind::Hunger,
                                            index,
                                        },
                                    ));
                                }
                            });
                        });

                    center.spawn((
                        Text::new("0"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgb(0.45, 1.0, 0.30)),
                        ExperienceText,
                    ));

                    center
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(6.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.05, 0.08, 0.04, 0.95)),
                        ))
                        .with_child((
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.90, 0.12)),
                            ExperienceFill,
                        ));
                });
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
                for slot_index in 0..HOTBAR_SLOT_COUNT {
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
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 0.9)),
                            BorderColor(if slot_index == 0 {
                                Color::WHITE
                            } else {
                                Color::srgba(0.12, 0.14, 0.18, 0.9)
                            }),
                            HotbarSlot(slot_index),
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(4.0),
                                    top: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new((slot_index + 1).to_string()),
                                TextFont::from_font_size(12.0),
                                TextColor(Color::WHITE),
                            ));
                            slot.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(4.0),
                                    bottom: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new(""),
                                TextFont::from_font_size(10.0),
                                TextColor(Color::WHITE),
                                HotbarCount(slot_index),
                            ));
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
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    select_buttons: Query<(&Interaction, &SelectWorldAction), Changed<Interaction>>,
    mut world_rows: Query<(&SelectWorldAction, &mut BackgroundColor)>,
    mut selection_status: Query<&mut Text, With<WorldSelectionStatus>>,
    mut selected: ResMut<SelectedWorld>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Back => next.set(AppState::MainMenu),
            MenuCommand::Create => {
                commands.insert_resource(WorldForm::default());
                next.set(AppState::CreateWorld);
            }
            MenuCommand::PlaySelected => {
                let Some(path) = selected.path.as_ref() else {
                    continue;
                };
                match load_world(path) {
                    Ok(save) => {
                        pending.save = Some(save);
                        pending.path = Some(path.clone());
                        pending.is_new_world = false;
                        *storage = WorldStorage::default();
                        next.set(AppState::LoadingWorld);
                    }
                    Err(error) => error!("Could not load world {:?}: {error}", path),
                }
            }
            MenuCommand::DeleteSelected if selected.path.is_some() => {
                next.set(AppState::ConfirmDeleteWorld);
            }
            MenuCommand::ConfirmDelete => {
                if let Some(path) = selected.path.as_ref() {
                    let result = fs::remove_file(path)
                        .map_err(|error| format!("No se pudo eliminar el mundo: {error}"))
                        .and_then(|()| {
                            let chunks = chunk_directory(path);
                            if chunks.exists() {
                                fs::remove_dir_all(&chunks).map_err(|error| {
                                    format!("No se pudieron eliminar los chunks: {error}")
                                })?;
                            }
                            Ok(())
                        });
                    match result {
                        Ok(()) => {
                            *selected = SelectedWorld::default();
                            next.set(AppState::WorldSelect);
                        }
                        Err(error) => {
                            error!("{error}");
                            next.set(AppState::WorldSelect);
                        }
                    }
                } else {
                    next.set(AppState::WorldSelect);
                }
            }
            MenuCommand::CancelDelete => next.set(AppState::WorldSelect),
            _ => {}
        }
    }
    for (interaction, action) in &select_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        selected.path = Some(action.path.clone());
        selected.name = Some(action.name.clone());
        for (row, mut background) in &mut world_rows {
            background.0 = if row.path == action.path {
                Color::srgb(0.18, 0.36, 0.29)
            } else {
                Color::srgb(0.12, 0.2, 0.25)
            };
        }
        for mut text in &mut selection_status {
            **text = format!("Seleccionado: {}", action.name).into();
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
                    terrain_version: TERRAIN_VERSION,
                    time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
                    player_position: [8.5, 64.0, 8.5],
                    player_yaw: 0.0,
                    player_pitch: -0.3,
                    selected_block: GRASS,
                    selected_hotbar_slot: 0,
                    inventory: [0; LEGACY_BLOCK_COUNT],
                    extra_inventory: vec![0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
                    slot_inventory: Some(Inventory::default()),
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
    mut streaming: ResMut<ChunkStreamingState>,
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
    storage.retry_generation = storage.retry_generation.wrapping_add(1);
    streaming.invalidate();
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
    world.terrain_version = save.terrain_version;
    day_night.time_of_day_seconds = save.time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS);
    let Some(saved_inventory) = save
        .slot_inventory
        .as_ref()
        .filter(|inventory| inventory.is_valid())
    else {
        error!("Loading was requested with invalid inventory data");
        storage.status = "El inventario guardado no es válido.".into();
        progress.initialized = false;
        next.set(AppState::MainMenu);
        return;
    };
    *inventory = saved_inventory.clone();
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
    selected_slot.0 = save.selected_hotbar_slot;
    selected.0 = match inventory.slots[selected_slot.0] {
        Some(ItemStack {
            item: Item::Block(block),
            ..
        }) => block,
        _ => AIR,
    };

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

    if progress.is_new_world {
        let (spawn_x, ground, spawn_z) = world_spawn_point(save.seed, save.terrain_version);
        save.player_position = [spawn_x, ground as f32 + 1.0 + PLAYER_EYE_HEIGHT, spawn_z];
    }

    let player_block = IVec3::new(
        save.player_position[0].floor() as i32,
        save.player_position[1].floor() as i32,
        save.player_position[2].floor() as i32,
    );
    let mut center = block_address(player_block).0;
    let vertical_radius = if world.terrain_version >= 2 {
        STREAM_VERTICAL_RADIUS
    } else {
        center.y = 0;
        0
    };
    progress.coordinates = chunk_coordinates_around(center, STARTUP_LOAD_RADIUS, vertical_radius);
    progress.chunk_count = progress.coordinates.len();
    progress.total = progress.chunk_count * 2;
}

fn chunk_coordinates_around(center: IVec3, radius: i32, vertical_radius: i32) -> Vec<IVec3> {
    let min_y = (center.y - vertical_radius).clamp(0, WORLD_HEIGHT / CHUNK_SIZE - 1);
    let max_y = (center.y + vertical_radius).clamp(0, WORLD_HEIGHT / CHUNK_SIZE - 1);
    let mut coordinates =
        Vec::with_capacity(((radius * 2 + 1).pow(2) * (max_y - min_y + 1)) as usize);
    for z in -radius..=radius {
        for x in -radius..=radius {
            for y in min_y..=max_y {
                coordinates.push(IVec3::new(center.x + x, y, center.z + z));
            }
        }
    }
    coordinates
}

fn loading_world_step(
    mut commands: Commands,
    mut progress: ResMut<LoadingProgress>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut streaming: ResMut<ChunkStreamingState>,
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
                        alpha_mode: AlphaMode::Mask(0.4),
                        metallic: 0.0,
                        perceptual_roughness: 1.0,
                        reflectance: 0.0,
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
                let entity = commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32),
                        ChunkMesh(coord),
                        GameEntity,
                    ))
                    .id();
                streaming.chunk_entities.insert(coord, entity);
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
    if crafting_ui.open || crafting_ui.inventory_open || crafting_ui.just_closed {
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
    let mut save: WorldSave = ron::from_str(&contents)
        .map_err(|error| format!("El archivo de mundo no es válido: {error}"))?;
    migrate_world_save(&mut save)?;
    validate_world_save(&save)?;
    Ok(save)
}

fn migrate_world_save(save: &mut WorldSave) -> Result<(), String> {
    if save.version != 1 {
        return Ok(());
    }

    // Version 1 saved block counts in legacy arrays. Rebuild the slot-based
    // inventory so these worlds remain visible and playable in the current UI.
    if save.slot_inventory.is_none() {
        let mut inventory = Inventory::default();
        for (block, count) in save.inventory.iter().copied().enumerate() {
            if block != AIR as usize && count > 0 {
                inventory
                    .try_add(Item::Block(block as u8), count)
                    .map_err(|error| format!("No se pudo migrar el inventario: {error}"))?;
            }
        }
        for (index, count) in save.extra_inventory.iter().copied().enumerate() {
            let block = index + LEGACY_BLOCK_COUNT;
            if count > 0 && block < BLOCK_COUNT {
                inventory
                    .try_add(Item::Block(block as u8), count)
                    .map_err(|error| format!("No se pudo migrar el inventario: {error}"))?;
            }
        }
        save.slot_inventory = Some(inventory);
    }
    save.version = SAVE_VERSION;
    Ok(())
}

fn validate_world_save(save: &WorldSave) -> Result<(), String> {
    if save.version != SAVE_VERSION {
        return Err(format!("Versión de mundo no compatible: {}", save.version));
    }
    if save.terrain_version == 0 || save.terrain_version > TERRAIN_VERSION {
        return Err(format!(
            "Versión de terreno no compatible: {}",
            save.terrain_version
        ));
    }
    if save.name.trim().is_empty() || save.name.chars().count() > 32 {
        return Err("El nombre del mundo no es válido.".into());
    }
    if save.selected_hotbar_slot >= HOTBAR_SLOT_COUNT {
        return Err("El bloque seleccionado no es válido.".into());
    }
    if save.selected_block >= BLOCK_COUNT as u8 {
        return Err("El bloque seleccionado no es válido.".into());
    }
    if !save
        .slot_inventory
        .as_ref()
        .is_some_and(Inventory::is_valid)
    {
        return Err("El inventario guardado no es válido.".into());
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
    if save.pickaxe_durability > 64
        || save.axe_durability > 64
        || save.shovel_durability > 64
        || save.sword_durability > 32
    {
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
        if chunk.blocks.len() != CHUNK_VOLUME
            || chunk.blocks.iter().any(|block| *block >= BLOCK_COUNT as u8)
        {
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

fn is_valid_block(block: u8) -> bool {
    block < BLOCK_COUNT as u8
}

fn block_name(block: u8) -> &'static str {
    block_definition(block).name_es
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
    block_definition(block).experience
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
    let legacy_inventory = std::array::from_fn(|index| inventory.count(index as u8));
    let extra_inventory = (LEGACY_BLOCK_COUNT..BLOCK_COUNT)
        .map(|index| inventory.count(index as u8))
        .collect();
    WorldSave {
        version: SAVE_VERSION,
        name: world.name.clone(),
        seed: world.seed,
        terrain_version: world.terrain_version,
        time_of_day_seconds,
        player_position: player_position.to_array(),
        player_yaw: player.yaw,
        player_pitch: player.pitch,
        selected_block,
        selected_hotbar_slot,
        inventory: legacy_inventory,
        extra_inventory,
        slot_inventory: Some(inventory.clone()),
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
    if chunk.blocks.len() != CHUNK_VOLUME
        || chunk.blocks.iter().any(|block| *block >= BLOCK_COUNT as u8)
    {
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
    let was_retrying = storage.retry_elapsed > 0.0;
    storage.retry_elapsed = (storage.retry_elapsed - time.delta_secs()).max(0.0);
    if was_retrying && storage.retry_elapsed == 0.0 && !storage.failed_chunks.is_empty() {
        storage.failed_chunks.clear();
        storage.retry_generation = storage.retry_generation.wrapping_add(1);
    }
    let Some(path) = storage.path.clone() else {
        return;
    };
    storage.autosave_elapsed += time.delta_secs();
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
fn block_drop_color(block_type: u8) -> Color {
    let color = block_definition(block_type).drop_color;
    Color::linear_rgb(color[0], color[1], color[2])
}

fn item_drop_color(item: Item) -> Color {
    match item {
        Item::Block(block) => block_drop_color(block),
        Item::Mineral(MineralKind::Coal) => Color::srgb(0.12, 0.12, 0.13),
        Item::Mineral(MineralKind::RawCopper) => Color::srgb(0.78, 0.42, 0.24),
        Item::Mineral(MineralKind::RawIron) => Color::srgb(0.70, 0.56, 0.45),
        Item::Tool { .. } => Color::srgb(0.55, 0.58, 0.62),
        Item::BerryRation => Color::srgb(0.42, 0.62, 0.28),
        Item::Arrow => Color::srgb(0.72, 0.68, 0.55),
    }
}

fn inventory_item_for_block_drop(drop: BlockDropDefinition) -> Item {
    match drop {
        BlockDropDefinition::Block(block) => Item::Block(block),
        BlockDropDefinition::Mineral(mineral) => Item::Mineral(mineral),
    }
}

fn spawn_block_drop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    item: Item,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.28, 0.28, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: item_drop_color(item),
            ..default()
        })),
        Transform::from_translation(position),
        BlockDrop {
            item,
            velocity: Vec3::new(0.0, 2.0, 0.0),
            age: 0.0,
        },
    ));
}

fn update_block_drops(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<WorldData>,
    player: Query<&Transform, With<Player>>,
    mut inventory: ResMut<Inventory>,
    mut drops: Query<(Entity, &mut BlockDrop, &mut Transform), Without<Player>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let dt = time.delta_secs();
    // The player's Transform is at eye height; use the body center for pickup range.
    let pickup_target = player.translation - Vec3::Y * (PLAYER_EYE_HEIGHT - 0.5);
    for (entity, mut drop, mut transform) in &mut drops {
        drop.age += dt;
        let to_player = pickup_target - transform.translation;
        let distance = to_player.length();
        if drop.age > 0.45 && distance <= 1.9 {
            transform.translation += to_player.normalize_or_zero() * (6.0 * dt).min(distance);
            if transform.translation.distance(pickup_target) <= 0.35
                && inventory.try_add(drop.item, 1).is_ok()
            {
                commands.entity(entity).despawn();
                continue;
            }
        } else {
            let below = (transform.translation - Vec3::Y * 0.2).floor().as_ivec3();
            if block_at(&world, below) != AIR {
                drop.velocity.y = drop.velocity.y.max(0.0);
                transform.translation.y = transform.translation.y.max(below.y as f32 + 1.14);
            } else {
                drop.velocity.y -= 12.0 * dt;
                transform.translation.y += drop.velocity.y * dt;
            }
        }
        transform.rotate_y(dt * 1.8);
    }
}

fn spawn_block_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    block_pos: Vec3,
    block_type: u8,
) {
    // Spawn 5 particles per destroyed block
    let particle_count = 5;
    let block_color = block_drop_color(block_type);

    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let speed = 2.0 + (i as f32) * 0.3;
        let velocity = Vec3::new(
            angle.cos() * speed,
            3.0 + (i as f32) * 0.2,
            angle.sin() * speed,
        );

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
            material.base_color =
                Color::linear_rgb(0.2 * brightness, 0.5 * brightness, 0.8 * brightness);
        }
    }

    if let Some(lava_handle) = &texture_assets.lava_material {
        if let Some(material) = materials.get_mut(lava_handle) {
            // Lava: cycle between orange and slightly darker orange
            material.base_color =
                Color::linear_rgb(1.0 * brightness, 0.5 * brightness, 0.0 * brightness);
        }
    }
}

fn chat_input(
    mut events: EventReader<KeyboardInput>,
    mut chat: ResMut<ChatState>,
    crafting_ui: Res<CraftingUi>,
    mut cycle: ResMut<DayNightCycle>,
    mut survival: ResMut<Survival>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let opens_chat = matches!(
            &event.logical_key,
            Key::Character(character) if character.eq_ignore_ascii_case("t")
        );
        if !chat.open {
            if opens_chat && !crafting_ui.open && !crafting_ui.inventory_open {
                chat.open = true;
                chat.input.clear();
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::None;
                    window.cursor_options.visible = true;
                }
            }
            continue;
        }

        match &event.logical_key {
            Key::Escape => {
                chat.open = false;
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
            }
            Key::Enter => {
                let command = chat.input.trim().to_ascii_lowercase();
                match command.as_str() {
                    "/timesetday" => {
                        cycle.time_of_day_seconds = 12.0 * 60.0 * 60.0;
                        survival.notice = "Hora establecida: día.".into();
                        survival.notice_elapsed = 3.0;
                    }
                    "" => {}
                    _ if !command.starts_with('/') => {
                        survival.notice = "Los comandos empiezan con /.".into();
                        survival.notice_elapsed = 3.0;
                    }
                    _ => {
                        survival.notice = "Comando desconocido.".into();
                        survival.notice_elapsed = 3.0;
                    }
                }
                chat.open = false;
                chat.input.clear();
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
            }
            Key::Backspace => {
                chat.input.pop();
            }
            Key::Character(character)
                if chat.input.chars().count() < 128 && !character.chars().any(char::is_control) =>
            {
                chat.input.push_str(character);
            }
            _ => {}
        }
    }
}

fn update_chat_bar(
    chat: Res<ChatState>,
    mut bar: Query<&mut Node, With<ChatBar>>,
    mut input: Query<&mut Text, With<ChatInputText>>,
) {
    if let Ok(mut bar) = bar.get_single_mut() {
        bar.display = if chat.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut input) = input.get_single_mut() {
        input.0 = format!("> {}", chat.input);
    }
}

fn capture_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    crafting_ui: Res<CraftingUi>,
    chat: Res<ChatState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
) {
    pointer.captured_this_frame = false;
    if crafting_ui.open || crafting_ui.inventory_open || chat.open {
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
    chat: Res<ChatState>,
) {
    if chat.open {
        return;
    }
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
    time: Res<Time<Fixed>>,
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

    let delta = time.delta_secs();
    let jump_pressed = keys.pressed(KeyCode::Space) && !player.jump_key_down;
    let flight_pressed = keys.pressed(KeyCode::KeyF) && !player.flight_key_down;
    player.jump_key_down = keys.pressed(KeyCode::Space);
    player.flight_key_down = keys.pressed(KeyCode::KeyF);

    if flight_pressed {
        player.flying = !player.flying;
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        player.jump_buffer = 0.0;
        player.coyote_time = 0.0;
        player.step_up_remaining = 0.0;
        info!(
            "Movement mode: {}",
            if player.flying { "flight" } else { "walking" }
        );
    }

    if !player.flying {
        let safe_position = find_safe_spawn_position(&world, transform.translation);
        if safe_position != transform.translation {
            transform.translation = safe_position;
            player.velocity = Vec3::ZERO;
            player.grounded = false;
            player.jump_buffer = 0.0;
            player.coyote_time = 0.0;
            player.step_up_remaining = 0.0;
        }
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

    if player.flying {
        if keys.pressed(KeyCode::Space) {
            direction.y += 1.0;
        }
        if keys.pressed(KeyCode::ControlLeft) {
            direction.y -= 1.0;
        }
        let target_velocity = direction.normalize_or_zero() * SPRINT_SPEED;
        let control_rate = if direction == Vec3::ZERO {
            FLIGHT_FRICTION
        } else {
            FLIGHT_ACCELERATION
        };
        player.velocity = approach_velocity(player.velocity, target_velocity, control_rate * delta);
        transform.translation += player.velocity * delta;
        return;
    }

    let swimming = block_at(
        &world,
        (transform.translation - Vec3::Y * 0.9).floor().as_ivec3(),
    ) == WATER;
    let speed = if swimming {
        WALK_SPEED * 0.7
    } else if keys.pressed(KeyCode::ShiftLeft) {
        SPRINT_SPEED
    } else {
        WALK_SPEED
    };
    let input_direction = direction.normalize_or_zero();
    let target_velocity = input_direction * speed;
    let horizontal_velocity = Vec2::new(player.velocity.x, player.velocity.z);
    let target_horizontal_velocity = Vec2::new(target_velocity.x, target_velocity.z);
    let control_rate = if player.grounded {
        if input_direction == Vec3::ZERO {
            GROUND_FRICTION
        } else {
            GROUND_ACCELERATION
        }
    } else if input_direction == Vec3::ZERO {
        AIR_DRAG
    } else {
        AIR_ACCELERATION
    };
    let velocity_delta = target_horizontal_velocity - horizontal_velocity;
    let horizontal_velocity = if velocity_delta.length() <= control_rate * delta {
        target_horizontal_velocity
    } else {
        horizontal_velocity + velocity_delta.normalize() * control_rate * delta
    };
    player.velocity.x = horizontal_velocity.x;
    player.velocity.z = horizontal_velocity.y;

    let was_grounded = player.grounded;
    if was_grounded {
        player.coyote_time = COYOTE_TIME_SECONDS;
    } else {
        player.coyote_time = (player.coyote_time - delta).max(0.0);
    }
    if jump_pressed {
        player.jump_buffer = JUMP_BUFFER_SECONDS;
    } else {
        player.jump_buffer = (player.jump_buffer - delta).max(0.0);
    }

    if swimming {
        player.jump_buffer = 0.0;
        if jump_pressed {
            player.velocity.y = 3.0;
        }
        player.velocity.y = (player.velocity.y - 4.0 * delta).max(-3.0);
    } else {
        if player.jump_buffer > 0.0 && player.coyote_time > 0.0 {
            player.velocity.y = JUMP_SPEED;
            player.jump_buffer = 0.0;
            player.coyote_time = 0.0;
            player.grounded = false;
            player.step_up_remaining = 0.0;
        }
        if player.step_up_remaining > 0.0 {
            player.velocity.y = 0.0;
        } else {
            player.velocity.y = (player.velocity.y - GRAVITY * delta).max(-50.0);
        }
    }

    let displacement = player.velocity * delta;
    let step_count = (displacement.abs().max_element() / 0.05).ceil().max(1.0) as usize;
    let step = displacement / step_count as f32;
    let can_step = was_grounded || player.step_up_remaining > 0.0;
    player.grounded = false;

    for _ in 0..step_count {
        for axis in [0, 2] {
            let mut candidate = transform.translation;
            candidate[axis] += step[axis];
            if !collides_with_world(&world, candidate) {
                transform.translation = candidate;
            } else if can_step && step[axis] != 0.0 && player.step_up_remaining == 0.0 {
                let raised = transform.translation + Vec3::Y * STEP_HEIGHT;
                let mut stepped = raised;
                stepped[axis] += step[axis];
                if !collides_with_world(&world, raised) && !collides_with_world(&world, stepped) {
                    player.step_up_remaining = STEP_HEIGHT;
                    player.velocity.y = 0.0;
                }
            }
        }
    }

    if player.step_up_remaining > 0.0 {
        let lift = (STEP_UP_SPEED * delta).min(player.step_up_remaining);
        let mut candidate = transform.translation;
        candidate.y += lift;
        if !collides_with_world(&world, candidate) {
            transform.translation = candidate;
            player.step_up_remaining -= lift;
            player.grounded = true;
            player.velocity.y = 0.0;
        } else {
            player.step_up_remaining = 0.0;
        }
    } else {
        for _ in 0..step_count {
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
}

fn approach_velocity(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let difference = target - current;
    if difference.length() <= max_delta {
        target
    } else {
        current + difference.normalize_or_zero() * max_delta
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
        let (spawn_x, ground, spawn_z) = world_spawn_point(world.seed, world.terrain_version);
        transform.translation =
            Vec3::new(spawn_x, ground as f32 + 1.0 + PLAYER_EYE_HEIGHT, spawn_z);
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        player.flying = false;
        player.jump_buffer = 0.0;
        player.coyote_time = 0.0;
        player.step_up_remaining = 0.0;
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

fn consume_stored_food(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut survival: ResMut<Survival>,
) {
    if keys.just_pressed(KeyCode::KeyR)
        && windows
            .get_single()
            .is_ok_and(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
    {
        eat_food(&mut survival);
    }
}

fn stream_world_chunks(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut streaming: ResMut<ChunkStreamingState>,
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
        player.translation.y.floor() as i32,
        player.translation.z.floor() as i32,
    );
    let mut center = block_address(player_block).0;
    let vertical_radius = if world.terrain_version >= 2 {
        STREAM_VERTICAL_RADIUS
    } else {
        center.y = 0;
        0
    };

    let refresh_queue = streaming.center != Some(center)
        || streaming.vertical_radius != vertical_radius
        || streaming.retry_generation != storage.retry_generation;
    if refresh_queue {
        streaming.pending_unloads = chunk_entities
            .iter()
            .filter_map(|(entity, chunk)| {
                let coord = chunk.0;
                let distance = (i64::from(coord.x) - i64::from(center.x))
                    .abs()
                    .max((i64::from(coord.z) - i64::from(center.z)).abs());
                let vertical_distance = (i64::from(coord.y) - i64::from(center.y)).abs();
                (distance > i64::from(STREAM_UNLOAD_RADIUS)
                    || vertical_distance > i64::from(vertical_radius))
                .then_some((entity, coord))
                .filter(|(_, coord)| !storage.failed_chunks.contains(coord))
            })
            .collect();
        streaming.next_unload = 0;
        streaming.pending = chunk_coordinates_around(center, STREAM_LOAD_RADIUS, vertical_radius)
            .into_iter()
            .filter(|coord| {
                !world.chunks.contains_key(coord) && !storage.failed_chunks.contains(coord)
            })
            .collect();
        streaming.pending.sort_unstable_by_key(|coord| {
            let x = i64::from(coord.x) - i64::from(center.x);
            let y = i64::from(coord.y) - i64::from(center.y);
            let z = i64::from(coord.z) - i64::from(center.z);
            (x * x + y * y + z * z, coord.x, coord.y, coord.z)
        });
        streaming.center = Some(center);
        streaming.vertical_radius = vertical_radius;
        streaming.retry_generation = storage.retry_generation;
        streaming.next = 0;
    }

    let mut unloaded = 0;
    while unloaded < MAX_CHUNK_UNLOADS_PER_FRAME {
        let Some((entity, coord)) = streaming
            .pending_unloads
            .get(streaming.next_unload)
            .copied()
        else {
            break;
        };
        streaming.next_unload += 1;
        if chunk_entities.get(entity).is_err() {
            continue;
        }
        if let Some(chunk) = world.modified_chunks.get(&coord) {
            let Some(path) = storage.path.as_ref() else {
                storage.status = "No se encontró la ruta para guardar un chunk.".into();
                storage.failed_chunks.insert(coord);
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
        streaming.chunk_entities.remove(&coord);
        world.chunks.remove(&coord);
        world.dirty.remove(&coord);
        mark_chunk_neighbors_dirty(&mut world, coord);
        unloaded += 1;
    }

    let coord = loop {
        let Some(coord) = streaming.pending.get(streaming.next).copied() else {
            return;
        };
        streaming.next += 1;
        if !world.chunks.contains_key(&coord) && !storage.failed_chunks.contains(&coord) {
            break coord;
        }
    };
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
            alpha_mode: AlphaMode::Mask(0.4),
            metallic: 0.0,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        }));
    }
    let Some(material) = loading.material.clone() else {
        error!("Could not create the chunk material");
        world.chunks.remove(&coord);
        return;
    };
    let mesh = meshes.add(rendering::build_chunk_mesh(&world, coord));
    let entity = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32),
            ChunkMesh(coord),
            GameEntity,
        ))
        .id();
    streaming.chunk_entities.insert(coord, entity);
}

fn mark_chunk_neighbors_dirty(world: &mut WorldData, coord: IVec3) {
    for axis in [0, 1, 2] {
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
        || {
            if world.terrain_version >= TERRAIN_VERSION {
                generate_chunk(coord, world.seed)
            } else if world.terrain_version >= 2 {
                generate_v2_chunk(coord, world.seed)
            } else if coord.y == 0 {
                generate_legacy_chunk(coord, world.seed)
            } else {
                Chunk::new()
            }
        },
        |chunk| Chunk {
            blocks: chunk.blocks.clone(),
        },
    )
}

#[cfg(feature = "plains-audio")]
fn update_plains_ambience(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<WorldData>,
    player: Query<&Transform, With<Player>>,
    ambience: Query<Entity, With<PlainsAmbience>>,
) {
    let in_plains = player.get_single().is_ok_and(|transform| {
        world_biome_at(
            &world,
            transform.translation.x.floor() as i32,
            transform.translation.z.floor() as i32,
        ) == Biome::Plains
    });
    if in_plains && ambience.is_empty() {
        commands.spawn((
            AudioPlayer::new(asset_server.load("audio/plains_ambience.wav")),
            PlaybackSettings::LOOP.with_volume(Volume::new(0.22)),
            PlainsAmbience,
        ));
    } else if !in_plains {
        for entity in &ambience {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[cfg(feature = "plains-audio")]
fn cleanup_plains_ambience(mut commands: Commands, ambience: Query<Entity, With<PlainsAmbience>>) {
    for entity in &ambience {
        commands.entity(entity).despawn_recursive();
    }
}

#[cfg(not(feature = "plains-audio"))]
fn update_plains_ambience() {}

#[cfg(not(feature = "plains-audio"))]
fn cleanup_plains_ambience() {}

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
    if kind == MobKind::Grazer
        && world_biome_at(&world, position.x.floor() as i32, position.z.floor() as i32)
            == Biome::Plains
    {
        let herd_size = 2
            + (coordinate_hash(
                position.x as i32,
                position.z as i32,
                world.seed ^ population.spawn_attempt,
            ) % 2) as usize;
        let offsets = [(2, 1), (-2, 1), (1, -2), (-1, -2)];
        for (index, (dx, dz)) in offsets.into_iter().take(herd_size - 1).enumerate() {
            if grazer_count + index + 1 >= MAX_PASSIVE_MOBS || total_count + index + 1 >= MAX_MOBS {
                break;
            }
            let x = position.x.floor() as i32 + dx;
            let z = position.z.floor() as i32 + dz;
            if world_biome_at(&world, x, z) == Biome::Plains {
                if let Some(member_position) = mob_ground_position(&world, x, z) {
                    spawn_mob(&mut commands, &assets, MobKind::Grazer, member_position);
                }
            }
        }
    }
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

fn find_safe_spawn_position(world: &WorldData, mut position: Vec3) -> Vec3 {
    while collides_with_world(world, position) {
        let next_y = (position.y - PLAYER_EYE_HEIGHT + 0.0001).floor() + 1.0 + PLAYER_EYE_HEIGHT;
        if next_y <= position.y {
            error!("Cannot recover player from collision at {:?}", position);
            break;
        }
        position.y = next_y;
    }
    position
}

fn mob_ground_height(world: &WorldData, x: i32, z: i32) -> Option<i32> {
    let chunk_x = x.div_euclid(CHUNK_SIZE);
    let chunk_z = z.div_euclid(CHUNK_SIZE);
    if !world
        .chunks
        .keys()
        .any(|coord| coord.x == chunk_x && coord.z == chunk_z)
    {
        return None;
    }
    (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| is_block_solid(world, IVec3::new(x, y, z)))
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
    let biome = world_biome_at(world, x, z);
    if !matches!(biome, Biome::Plains | Biome::Forest) {
        return None;
    }

    mob_ground_position(world, x, z)
}

fn mob_ground_position(world: &WorldData, x: i32, z: i32) -> Option<Vec3> {
    let ground = mob_ground_height(world, x, z)?;
    if block_at(world, IVec3::new(x, ground, z)) != GRASS
        || !is_mob_clear_block(block_at(world, IVec3::new(x, ground + 1, z)))
        || !is_mob_clear_block(block_at(world, IVec3::new(x, ground + 2, z)))
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

fn is_mob_clear_block(block: u8) -> bool {
    matches!(block, AIR | RED_FLOWER | YELLOW_FLOWER | TALL_GRASS)
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
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    ui: Res<CraftingUi>,
    chat: Res<ChatState>,
) {
    if ui.open || ui.inventory_open || chat.open {
        return;
    }
    let slot = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .position(|key| keys.just_pressed(*key));
    if let Some(slot) = slot {
        selected_slot.0 = slot;
    }
}

fn select_hotbar(
    mut wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
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
    selected_slot.0 = slot;
}

fn sync_selected_block(
    inventory: Res<Inventory>,
    selected_slot: Res<SelectedHotbarSlot>,
    mut selected: ResMut<SelectedBlock>,
) {
    selected.0 = match inventory.slots[selected_slot.0] {
        Some(ItemStack {
            item: Item::Block(block),
            ..
        }) => block,
        _ => AIR,
    };
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

fn update_survival_hud(
    survival: Res<Survival>,
    mut icons: Query<(&SurvivalIcon, &mut Text, &mut TextColor), Without<ExperienceText>>,
    mut experience_fill: Query<&mut Node, With<ExperienceFill>>,
    mut experience_text: Query<&mut Text, (With<ExperienceText>, Without<SurvivalIcon>)>,
) {
    let (level, progress, required) = experience_progress(survival.experience);
    for (icon, mut text, mut color) in &mut icons {
        let amount = match icon.kind {
            SurvivalIconKind::Health => survival.health,
            SurvivalIconKind::Hunger => survival.hunger,
        };
        let fill = (amount - icon.index as f32 * 2.0).clamp(0.0, 2.0);
        let half = fill >= 1.0 && fill < 2.0;
        let full = fill >= 2.0;
        let (symbol, tint) = match (icon.kind, full, half) {
            (SurvivalIconKind::Health, true, _) => ("♥", Color::srgb(0.92, 0.16, 0.18)),
            (SurvivalIconKind::Health, false, true) => ("♥", Color::srgb(1.0, 0.46, 0.42)),
            (SurvivalIconKind::Health, false, false) => ("♡", Color::srgb(0.34, 0.18, 0.19)),
            (SurvivalIconKind::Hunger, true, _) => ("●", Color::srgb(0.88, 0.58, 0.18)),
            (SurvivalIconKind::Hunger, false, true) => ("◐", Color::srgb(0.88, 0.58, 0.18)),
            (SurvivalIconKind::Hunger, false, false) => ("○", Color::srgb(0.32, 0.29, 0.25)),
        };
        text.0 = symbol.into();
        color.0 = tint;
    }
    if let Ok(mut fill) = experience_fill.get_single_mut() {
        fill.width = Val::Percent(progress as f32 / required as f32 * 100.0);
    }
    if let Ok(mut text) = experience_text.get_single_mut() {
        text.0 = format!("{level}");
    }
}

fn format_time_of_day(time_of_day_seconds: f32) -> String {
    let total_minutes = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / 60.0).floor() as u32;
    format!("{:02}:{:02}", (total_minutes / 60) % 24, total_minutes % 60)
}

fn update_hotbar(
    selected: Res<SelectedHotbarSlot>,
    inventory: Res<Inventory>,
    mut slots: Query<(
        &HotbarSlot,
        &mut BorderColor,
        &mut Node,
        &mut BackgroundColor,
    )>,
    mut counts: Query<(&HotbarCount, &mut Text)>,
) {
    let selection_changed = selected.is_changed();
    let inventory_changed = inventory.is_changed();
    if !selection_changed && !inventory_changed {
        return;
    }

    for (slot, mut border_color, mut node, mut background) in &mut slots {
        if selection_changed {
            let is_selected = slot.0 == selected.0;
            border_color.0 = if is_selected {
                Color::WHITE
            } else {
                Color::srgba(0.12, 0.14, 0.18, 0.9)
            };
            node.border = UiRect::all(Val::Px(if is_selected { 3.0 } else { 1.0 }));
        }
        if inventory_changed {
            background.0 = match inventory.slots[slot.0].map(|stack| stack.item) {
                Some(Item::Block(GRASS)) => Color::srgb(0.25, 0.62, 0.22),
                Some(Item::Block(DIRT)) => Color::srgb(0.43, 0.30, 0.18),
                Some(Item::Block(STONE)) => Color::srgb(0.48, 0.50, 0.53),
                Some(Item::Block(SAND)) => Color::srgb(0.82, 0.72, 0.45),
                Some(Item::Block(WOOD)) => Color::srgb(0.48, 0.29, 0.12),
                Some(Item::Block(LEAVES)) => Color::srgb(0.16, 0.48, 0.19),
                Some(_) => Color::srgb(0.28, 0.30, 0.34),
                None => Color::srgba(0.12, 0.14, 0.18, 0.9),
            };
        }
    }
    if inventory_changed {
        for (slot, mut text) in &mut counts {
            text.0 = inventory.slots[slot.0].map_or_else(String::new, |stack| {
                let name = match stack.item {
                    Item::Block(WOODEN_DOOR) => "Puerta",
                    Item::Block(STAIRS) => "Escal.",
                    Item::Block(BRICKS) => "Ladr.",
                    Item::Block(PLANKS) => "Tablon",
                    Item::Block(MUSHROOM) => "Seta",
                    Item::Block(TORCH) => "Antor.",
                    item => item_name(item),
                };
                format!("{name}\n{}", stack.count)
            });
        }
    }
}

fn edit_blocks(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    (time, mut mining): (Res<Time>, ResMut<MiningState>),
    camera: Query<&Transform, With<Player>>,
    mut mobs: Query<(Entity, &Transform, &mut Mob), Without<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
    crafting_ui: Res<CraftingUi>,
    chat: Res<ChatState>,
    (selected, selected_slot): (Res<SelectedBlock>, Res<SelectedHotbarSlot>),
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if pointer.captured_this_frame || crafting_ui.open || crafting_ui.inventory_open || chat.open {
        mining.target = None;
        return;
    }
    let place = mouse.just_pressed(MouseButton::Right);
    let remove = mouse.pressed(MouseButton::Left) && !place;
    if !remove && !place {
        mining.target = None;
        return;
    }
    let Ok(window) = windows.get_single() else {
        mining.target = None;
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        mining.target = None;
        return;
    }
    let Ok(transform) = camera.get_single() else {
        mining.target = None;
        return;
    };

    if place {
        mining.target = None;
        if inventory.slots[selected_slot.0].is_some_and(|stack| stack.item == Item::BerryRation)
            && survival.hunger < MAX_HUNGER
        {
            item_inventory::take_one(&mut inventory.slots[selected_slot.0]);
            survival.hunger =
                (survival.hunger + FoodType::BerryRation.saturation()).min(MAX_HUNGER);
            survival.notice = "Comiste una ración.".into();
            survival.notice_elapsed = 2.0;
            return;
        }
    }
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
                mining.target = None;
                if !mouse.just_pressed(MouseButton::Left) {
                    return;
                }
                let Ok((_, _, mut mob)) = mobs.get_mut(entity) else {
                    return;
                };
                let sword = matches!(
                    inventory.slots[selected_slot.0],
                    Some(ItemStack {
                        item: Item::Tool {
                            kind: ToolKind::Sword,
                            ..
                        },
                        ..
                    })
                );
                let remaining = if sword {
                    wear_selected_tool(&mut inventory, selected_slot.0)
                } else {
                    None
                };
                let damage = if sword { 2 } else { 1 };
                mob.health = mob.health.saturating_sub(damage);
                if mob.health == 0 {
                    commands.entity(entity).despawn_recursive();
                    let (food, experience) = mob_rewards(kind);
                    survival.food = survival.food.saturating_add(food).min(99);
                    survival.experience = survival.experience.saturating_add(experience);
                    if remaining == Some(0) {
                        survival.notice = "La espada se rompió.".into();
                        survival.notice_elapsed = 4.0;
                    } else {
                        survival.notice = match kind {
                            MobKind::Grazer if food > 0 => {
                                "Conseguiste carne y experiencia.".into()
                            }
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
        mining.target = None;
        return;
    };

    if remove {
        let block = block_at(&world, hit);
        let tool = selected_mining_tool(block, inventory.slots[selected_slot.0]);
        if block_definition(block).requires_pickaxe && tool != Some(MiningTool::Pickaxe) {
            mining.target = None;
            survival.notice = "Necesitas un pico para extraer este mineral.".into();
            survival.notice_elapsed = 2.0;
            return;
        }
        let duration = mining_duration(block, tool);
        let target = mining.target.get_or_insert(MiningTarget {
            position: hit,
            block,
            tool,
            elapsed: 0.0,
            duration,
        });
        if target.position != hit || target.block != block || target.tool != tool {
            *target = MiningTarget {
                position: hit,
                block,
                tool,
                elapsed: 0.0,
                duration,
            };
        }
        target.elapsed += time.delta_secs();
        if target.elapsed < target.duration {
            return;
        }
        mining.target = None;
    }

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
            chunk.set(local, AIR);
            mined_block = Some(current_block);
            if let Some(drop) = block_definition(current_block).drop {
                spawn_block_drop(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target.as_vec3() + Vec3::splat(0.5),
                    inventory_item_for_block_drop(drop),
                );
            }
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
                || !inventory.slots[selected_slot.0]
                    .is_some_and(|stack| stack.item == Item::Block(selected.0))
                || block_overlaps_player(target, transform.translation)
            {
                return;
            }
            item_inventory::take_one(&mut inventory.slots[selected_slot.0]);
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
        if selected_mining_tool(block, inventory.slots[selected_slot.0]).is_some() {
            let durability = wear_selected_tool(&mut inventory, selected_slot.0)
                .expect("selected mining tool exists");
            if let Some(drop) = block_definition(block).drop {
                spawn_block_drop(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target.as_vec3() + Vec3::splat(0.5) + Vec3::Y * 0.15,
                    inventory_item_for_block_drop(drop),
                );
            }
            survival.experience = survival
                .experience
                .saturating_add(experience_for_block(block));
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

fn selected_mining_tool(block: u8, stack: Option<ItemStack>) -> Option<MiningTool> {
    let kind = match stack?.item {
        Item::Tool { kind, .. } => kind,
        _ => return None,
    };
    match (block, kind) {
        (STONE | COAL_ORE | COPPER_ORE | IRON_ORE, ToolKind::Pickaxe) => Some(MiningTool::Pickaxe),
        (WOOD | LEAVES, ToolKind::Axe) => Some(MiningTool::Axe),
        (SAND | DIRT, ToolKind::Shovel) => Some(MiningTool::Shovel),
        _ => None,
    }
}

fn wear_selected_tool(inventory: &mut Inventory, slot: usize) -> Option<u32> {
    let stack = inventory.slots.get_mut(slot)?.as_mut()?;
    let Item::Tool {
        ref mut durability, ..
    } = stack.item
    else {
        return None;
    };
    *durability -= 1;
    let remaining = *durability;
    if remaining == 0 {
        inventory.slots[slot] = None;
    }
    Some(remaining)
}

fn mining_duration(block: u8, tool: Option<MiningTool>) -> f32 {
    let base = block_definition(block).hardness;
    if tool.is_some() {
        base / 2.0
    } else {
        base
    }
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
    streaming: Res<ChunkStreamingState>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_entities: Query<&Mesh3d, With<ChunkMesh>>,
) {
    if world.dirty.is_empty() {
        return;
    }
    let mut dirty_chunks = Vec::with_capacity(MAX_DIRTY_CHUNK_MESH_REBUILDS_PER_FRAME);
    let mut stale_chunks = Vec::new();
    for coord in world.dirty.iter().copied() {
        let Some(entity) = streaming.chunk_entities.get(&coord) else {
            stale_chunks.push(coord);
            continue;
        };
        let Ok(mesh_handle) = chunk_entities.get(*entity) else {
            stale_chunks.push(coord);
            continue;
        };
        if meshes.get(&mesh_handle.0).is_none() {
            error!("Missing mesh asset for chunk {coord:?}");
            stale_chunks.push(coord);
            continue;
        }
        dirty_chunks.push((coord, mesh_handle.0.clone()));
        if dirty_chunks.len() == MAX_DIRTY_CHUNK_MESH_REBUILDS_PER_FRAME {
            break;
        }
    }
    for coord in stale_chunks {
        world.dirty.remove(&coord);
    }
    for (coord, mesh_handle) in dirty_chunks {
        let Some(mesh) = meshes.get_mut(&mesh_handle) else {
            continue;
        };
        *mesh = rendering::build_chunk_mesh(&world, coord);
        world.dirty.remove(&coord);
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
    if world.terrain_version >= 2 {
        if position.y < 0 {
            return STONE;
        }
        if position.y >= WORLD_HEIGHT {
            return AIR;
        }
    }
    let (chunk_coord, local) = block_address(position);
    world
        .chunks
        .get(&chunk_coord)
        .map_or(AIR, |chunk| chunk.get(local))
}

fn is_block_solid(world: &WorldData, position: IVec3) -> bool {
    let block = block_at(world, position);
    match block {
        WOODEN_DOOR => {
            // Open doors are not solid for collision
            let state = world
                .door_states
                .get(&position)
                .copied()
                .unwrap_or(DoorState::Closed);
            !matches!(state, DoorState::Open)
        }
        _ => block_definition(block).solid,
    }
}

fn light_level_at(world: &WorldData, position: IVec3) -> f32 {
    // Direct sunlight depends on height above surface
    let surface_height = world_terrain_height(world, position.x, position.z);
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

fn legacy_biome_at(x: i32, z: i32, seed: u64) -> Biome {
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

fn legacy_terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    let broad = value_noise(x, z, seed ^ 0x9E37_79B9_7F4A_7C15, 0.035);
    let detail = value_noise(x, z, seed ^ 0xBF58_476D_1CE4_E5B9, 0.11);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.012);
    let mountain_weight = smoothstep(0.64, 0.80, mountain);
    (4.0 + (broad - 0.5) * 8.0 + (detail - 0.5) * 2.0 + mountain_weight * 9.0)
        .round()
        .clamp(1.0, (CHUNK_SIZE - 1) as f32) as i32
}

fn biome_at(x: i32, z: i32, seed: u64) -> Biome {
    world::terrain::biome_at(x, z, seed)
}

fn biome_at_with_height_v2(x: i32, z: i32, seed: u64, height: i32) -> Biome {
    if (height - SEA_LEVEL).abs() <= 2 {
        return Biome::Beach;
    }

    let temperature = value_noise(x, z, seed ^ 0xA076_1D64_78BD_642F, 0.009);
    let moisture = value_noise(x, z, seed ^ 0xE703_7ED1_A0B4_28DB, 0.012);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.006);
    let mountain_weight = smoothstep(0.66, 0.86, mountain);
    let desert_weight =
        smoothstep(0.46, 0.62, temperature) * (1.0 - smoothstep(0.42, 0.60, moisture));
    let forest_weight = smoothstep(0.50, 0.68, moisture);

    if mountain_weight > 0.62 {
        Biome::Mountain
    } else if desert_weight > 0.22 {
        Biome::Desert
    } else if forest_weight > 0.38 {
        Biome::Forest
    } else {
        Biome::Plains
    }
}

fn world_biome_at(world: &WorldData, x: i32, z: i32) -> Biome {
    if world.terrain_version >= TERRAIN_VERSION {
        biome_at(x, z, world.seed)
    } else if world.terrain_version >= 2 {
        biome_at_with_height_v2(x, z, world.seed, terrain_height_v2(x, z, world.seed))
    } else {
        legacy_biome_at(x, z, world.seed)
    }
}

fn world_terrain_height(world: &WorldData, x: i32, z: i32) -> i32 {
    terrain_height_for_version(x, z, world.seed, world.terrain_version)
}

fn terrain_height_for_version(x: i32, z: i32, seed: u64, version: u32) -> i32 {
    if version >= TERRAIN_VERSION {
        terrain_height(x, z, seed)
    } else if version >= 2 {
        terrain_height_v2(x, z, seed)
    } else {
        legacy_terrain_height(x, z, seed)
    }
}

fn world_spawn_point(seed: u64, version: u32) -> (f32, i32, f32) {
    let center = IVec2::new(8, 8);
    for radius in (0_i32..=256).step_by(2) {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if radius > 0 && dx.abs() != radius && dz.abs() != radius {
                    continue;
                }
                let x = center.x + dx;
                let z = center.y + dz;
                let Some(height) = dry_spawn_column(seed, version, x, z) else {
                    continue;
                };
                if height <= SEA_LEVEL + 5 || height >= WORLD_HEIGHT - 4 {
                    continue;
                }
                if version >= TERRAIN_VERSION
                    && (!spawn_site_is_dry(seed, version, x, z)
                        || !spawn_site_has_relief(seed, x, z)
                        || !spawn_site_has_block_variety(seed, x, z))
                {
                    continue;
                }
                if height > SEA_LEVEL + 5 && height < WORLD_HEIGHT - 4 {
                    return (x as f32 + 0.5, height, z as f32 + 0.5);
                }
            }
        }
    }

    // The normal search also asks for local relief. Keep expanding until we
    // find a dry, layered site even if an unusual seed has an exceptionally
    // flat starting region.
    for radius in (258_i32..=4096).step_by(8) {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() != radius && dz.abs() != radius {
                    continue;
                }
                let x = center.x + dx;
                let z = center.y + dz;
                let Some(height) = dry_spawn_column(seed, version, x, z) else {
                    continue;
                };
                if height <= SEA_LEVEL + 5 || height >= WORLD_HEIGHT - 4 {
                    continue;
                }
                if version < TERRAIN_VERSION
                    || (spawn_site_is_dry(seed, version, x, z)
                        && spawn_site_has_block_variety(seed, x, z)
                        && spawn_site_has_relief(seed, x, z))
                {
                    return (x as f32 + 0.5, height, z as f32 + 0.5);
                }
            }
        }
    }

    panic!("No se encontró un punto de aparición seco y variado para la semilla {seed}");
}

fn dry_spawn_column(seed: u64, version: u32, x: i32, z: i32) -> Option<i32> {
    let (height, has_water) = if version >= TERRAIN_VERSION {
        let (height, water_level) = world::terrain::terrain_profile(x, z, seed);
        (height, water_level.is_some() || height < SEA_LEVEL)
    } else {
        let height = terrain_height_for_version(x, z, seed, version);
        (height, height < SEA_LEVEL)
    };
    (!has_water).then_some(height)
}

fn spawn_site_is_dry(seed: u64, version: u32, x: i32, z: i32) -> bool {
    (-1..=1).all(|dz| {
        (-1..=1).all(|dx| {
            dry_spawn_column(seed, version, x + dx, z + dz)
                .is_some_and(|height| height > SEA_LEVEL + 1)
        })
    })
}

fn spawn_site_has_relief(seed: u64, x: i32, z: i32) -> bool {
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;
    for (dx, dz) in [
        (-16, 0),
        (16, 0),
        (0, -16),
        (0, 16),
        (-12, -12),
        (-12, 12),
        (12, -12),
        (12, 12),
    ] {
        let height = terrain_height(x + dx, z + dz, seed);
        minimum = minimum.min(height);
        maximum = maximum.max(height);
    }
    maximum - minimum >= 2
}

fn spawn_site_has_block_variety(seed: u64, x: i32, z: i32) -> bool {
    let biome = biome_at(x, z, seed);
    let height = terrain_height(x, z, seed);
    if height <= SEA_LEVEL + 5 {
        return false;
    }
    // Reject solid-rock caps and require a real surface cap above deeper stone.
    !matches!(biome, Biome::Volcano | Biome::Cave)
        && (biome.surface_block() != STONE || biome == Biome::Mountain)
}

fn terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    world::terrain::terrain_height(x, z, seed)
}

fn terrain_height_v2(x: i32, z: i32, seed: u64) -> i32 {
    let continent = value_noise(x, z, seed ^ 0x9E37_79B9_7F4A_7C15, 0.0025);
    let rolling = value_noise(x, z, seed ^ 0xBF58_476D_1CE4_E5B9, 0.008);
    let detail = value_noise(x, z, seed ^ 0x94D0_49BB_1331_11EB, 0.04);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.006);
    let land_weight = smoothstep(0.34, 0.66, continent);
    let mountain_weight = smoothstep(0.68, 0.86, mountain);
    let mut height = (32.0
        + land_weight * 34.0
        + (rolling - 0.5) * 18.0
        + (detail - 0.5) * 4.0
        + mountain_weight * 42.0)
        .round()
        .clamp(8.0, (WORLD_HEIGHT - 16) as f32) as i32;

    let river = value_noise(x, z, seed ^ 0xD1B5_4A32_D192_ED03, 0.008);
    if (river - 0.5).abs() < 0.025 && height > SEA_LEVEL - 2 {
        height = SEA_LEVEL - 2;
    }
    height
}

fn is_cave_v2(x: i32, y: i32, z: i32, seed: u64) -> bool {
    if y <= 5 {
        return false;
    }
    let tunnel = value_noise_3d(x, y, z, seed ^ 0xC0FF_EE12_3456_7890, 0.035);
    let chamber = value_noise_3d(x, y, z, seed ^ 0x91E1_0DA5_C79E_7B1D, 0.065);
    (tunnel - 0.5).abs() < 0.045 && chamber > 0.38
}

fn is_cave(x: i32, y: i32, z: i32, seed: u64) -> bool {
    world::terrain::is_cave(x, y, z, seed)
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

fn value_noise_3d(x: i32, y: i32, z: i32, seed: u64, scale: f32) -> f32 {
    let scaled_x = x as f32 * scale;
    let scaled_y = y as f32 * scale;
    let scaled_z = z as f32 * scale;
    let grid_x = scaled_x.floor() as i32;
    let grid_y = scaled_y.floor() as i32;
    let grid_z = scaled_z.floor() as i32;
    let fraction_x = smoothstep(0.0, 1.0, scaled_x - grid_x as f32);
    let fraction_y = smoothstep(0.0, 1.0, scaled_y - grid_y as f32);
    let fraction_z = smoothstep(0.0, 1.0, scaled_z - grid_z as f32);
    let plane = |offset_y| {
        let top = lerp(
            hash_value_3d(grid_x, grid_y + offset_y, grid_z, seed),
            hash_value_3d(grid_x + 1, grid_y + offset_y, grid_z, seed),
            fraction_x,
        );
        let bottom = lerp(
            hash_value_3d(grid_x, grid_y + offset_y, grid_z + 1, seed),
            hash_value_3d(grid_x + 1, grid_y + offset_y, grid_z + 1, seed),
            fraction_x,
        );
        lerp(top, bottom, fraction_z)
    };
    lerp(plane(0), plane(1), fraction_y)
}

fn hash_value(x: i32, z: i32, seed: u64) -> f32 {
    (coordinate_hash(x, z, seed) >> 40) as f32 / (1_u32 << 24) as f32
}

fn hash_value_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (z as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value >> 40) as f32 / (1_u32 << 24) as f32
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
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::state::app::StatesPlugin;

    fn temporary_world_path() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("the-cubes-test-{}-{nonce}", std::process::id()))
            .join("world.ron")
    }

    fn mining_test_app(block: u8, tools: CraftedTools) -> (App, Entity, Entity) {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(0, 1, 2), block);
        world.chunks.insert(IVec3::ZERO, chunk);
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let visuals = MiningVisuals {
            materials: create_mining_materials(&mut images, &mut materials),
        };
        let overlay_material = visuals.materials[0].clone();
        let mut inventory = Inventory::default();
        for (kind, durability) in [
            (ToolKind::Pickaxe, tools.pickaxe_durability),
            (ToolKind::Axe, tools.axe_durability),
            (ToolKind::Shovel, tools.shovel_durability),
            (ToolKind::Sword, tools.sword_durability),
        ] {
            if durability > 0 {
                inventory
                    .try_add(Item::Tool { kind, durability }, 1)
                    .unwrap();
            }
        }
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(PointerState::default())
            .insert_resource(CraftingUi::default())
            .insert_resource(ChatState::default())
            .insert_resource(SelectedBlock::default())
            .insert_resource(inventory)
            .insert_resource(SelectedHotbarSlot::default())
            .insert_resource(Survival::default())
            .insert_resource(tools)
            .insert_resource(MiningState::default())
            .insert_resource(visuals)
            .insert_resource(world)
            .insert_resource(WorldStorage::default())
            .insert_resource(images)
            .insert_resource(materials)
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, (edit_blocks, update_mining_visual).chain());
        let mut window = Window::default();
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        app.world_mut().spawn((window, PrimaryWindow));
        let player = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.5, 1.5, 4.5),
                Player {
                    yaw: 0.0,
                    pitch: 0.0,
                    velocity: Vec3::ZERO,
                    grounded: false,
                    flying: false,
                    jump_buffer: 0.0,
                    coyote_time: 0.0,
                    step_up_remaining: 0.0,
                    jump_key_down: false,
                    flight_key_down: false,
                },
            ))
            .id();
        let overlay = app
            .world_mut()
            .spawn((
                Transform::default(),
                Visibility::Hidden,
                MeshMaterial3d(overlay_material),
                MiningOverlay,
            ))
            .id();
        (app, player, overlay)
    }

    fn mining_tick(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    }

    #[test]
    fn inventory_ui_moves_crafts_and_equips_items() {
        let mut app = App::new();
        let mut inventory = Inventory::default();
        inventory.try_add(Item::Block(WOOD), 1).unwrap();
        app.insert_resource(inventory)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<CraftingUi>()
            .init_resource::<ChatState>()
            .init_resource::<SelectedBlock>()
            .init_resource::<SelectedHotbarSlot>()
            .add_systems(
                Update,
                (
                    inventory_controls,
                    inventory_clicks,
                    sync_selected_block,
                    update_inventory_ui,
                )
                    .chain(),
            );
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut().spawn((Node::default(), InventoryPanel));
        let slot0 = app
            .world_mut()
            .spawn((Interaction::None, InventoryTarget::Slot(0)))
            .id();
        let grid0 = app
            .world_mut()
            .spawn((Interaction::None, InventoryTarget::Small(0)))
            .id();
        let result = app
            .world_mut()
            .spawn((Interaction::None, InventoryTarget::SmallResult))
            .id();
        let slot1 = app
            .world_mut()
            .spawn((Interaction::None, InventoryTarget::Slot(1)))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();
        assert!(app.world().resource::<CraftingUi>().inventory_open);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyE);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        for entity in [slot0, grid0, result, slot1] {
            *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Hovered;
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
            *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        }
        let inventory = app.world().resource::<Inventory>();
        assert_eq!(
            inventory.slots[1],
            Some(ItemStack {
                item: Item::Block(PLANKS),
                count: 4
            })
        );
        assert!(inventory.cursor.is_none());
        assert!(inventory.small_grid.iter().all(Option::is_none));
        app.world_mut().resource_mut::<SelectedHotbarSlot>().0 = 1;
        app.update();
        assert_eq!(app.world().resource::<SelectedBlock>().0, PLANKS);
    }

    #[test]
    fn world_save_restores_slots_cursor_grids_and_selected_tool() {
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemStack {
            item: Item::Tool {
                kind: ToolKind::Sword,
                durability: 15,
            },
            count: 1,
        });
        inventory.cursor = Some(ItemStack {
            item: Item::Block(STONE),
            count: 3,
        });
        inventory.small_grid[1] = Some(ItemStack {
            item: Item::Block(WOOD),
            count: 4,
        });
        inventory.large_grid[8] = Some(ItemStack {
            item: Item::Arrow,
            count: 5,
        });
        let player = Player {
            yaw: 0.0,
            pitch: 0.0,
            velocity: Vec3::ZERO,
            grounded: false,
            flying: false,
            jump_buffer: 0.0,
            coyote_time: 0.0,
            step_up_remaining: 0.0,
            jump_key_down: false,
            flight_key_down: false,
        };
        let world = WorldData {
            name: "Prueba".into(),
            ..default()
        };
        let save = snapshot_world(
            &world,
            Vec3::ZERO,
            &player,
            &inventory,
            AIR,
            3,
            DEFAULT_TIME_OF_DAY_SECONDS,
            &Survival::default(),
            &CraftedTools::default(),
        );
        let encoded = ron::to_string(&save).unwrap();
        let loaded: WorldSave = ron::from_str(&encoded).unwrap();
        validate_world_save(&loaded).unwrap();
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<LoadingProgress>()
            .init_resource::<WorldData>()
            .init_resource::<DayNightCycle>()
            .init_resource::<Inventory>()
            .init_resource::<Survival>()
            .init_resource::<CraftedTools>()
            .init_resource::<CraftingUi>()
            .init_resource::<SelectedBlock>()
            .init_resource::<SelectedHotbarSlot>()
            .init_resource::<WorldStorage>()
            .init_resource::<ChunkStreamingState>()
            .insert_resource(PendingWorld {
                save: Some(loaded),
                ..default()
            })
            .add_systems(Update, begin_loading_world);
        app.update();
        let restored = app.world().resource::<Inventory>();
        assert_eq!(restored.slots, inventory.slots);
        assert_eq!(restored.cursor, inventory.cursor);
        assert_eq!(restored.small_grid, inventory.small_grid);
        assert_eq!(restored.large_grid, inventory.large_grid);
        assert_eq!(app.world().resource::<SelectedHotbarSlot>().0, 3);
        assert_eq!(app.world().resource::<SelectedBlock>().0, AIR);
    }

    #[test]
    fn mining_requires_hold_and_shows_progress_until_the_block_breaks() {
        let (mut app, _, overlay) = mining_test_app(STONE, CraftedTools::default());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 2.9);
        let position = IVec3::new(0, 1, 2);
        assert_eq!(
            block_at(app.world().resource::<WorldData>(), position),
            STONE
        );
        assert_eq!(app.world().resource::<Inventory>().count(STONE), 0);
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            app.world().get::<Transform>(overlay).unwrap().translation,
            position.as_vec3() + Vec3::splat(0.5)
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(overlay)
                .unwrap()
                .0,
            app.world().resource::<MiningVisuals>().materials[7]
        );
        mining_tick(&mut app, 0.11);
        assert_eq!(block_at(app.world().resource::<WorldData>(), position), AIR);
        assert_eq!(app.world().resource::<Inventory>().count(STONE), 1);
        assert_eq!(app.world().resource::<Survival>().experience, 2);
        assert!(app
            .world()
            .resource::<WorldStorage>()
            .dirty_chunks
            .contains(&IVec3::ZERO));
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn mining_tools_halve_time_and_only_wear_after_a_successful_break() {
        let tools = CraftedTools {
            pickaxe_durability: 1,
            ..default()
        };
        let (mut app, _, _) = mining_test_app(STONE, tools);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 1.49);
        assert!(matches!(
            app.world().resource::<Inventory>().slots[0],
            Some(ItemStack {
                item: Item::Tool {
                    kind: ToolKind::Pickaxe,
                    durability: 1
                },
                ..
            })
        ));
        assert_eq!(app.world().resource::<Inventory>().count(STONE), 0);
        mining_tick(&mut app, 0.02);
        assert!(app.world().resource::<Inventory>().slots[0].is_none());
        assert_eq!(app.world().resource::<Inventory>().count(STONE), 2);
        assert_eq!(app.world().resource::<Survival>().experience, 4);
        assert_eq!(mining_duration(DIRT, Some(MiningTool::Shovel)), 0.4);
        assert_eq!(mining_duration(WOOD, Some(MiningTool::Axe)), 1.0);
        assert_eq!(mining_duration(STONE, None), 3.0);
        assert_eq!(
            selected_mining_tool(STONE, app.world().resource::<Inventory>().slots[0]),
            None
        );
    }

    #[test]
    fn mining_resets_on_release_target_change_and_open_inventory() {
        let (mut app, player, overlay) = mining_test_app(STONE, CraftedTools::default());
        app.world_mut()
            .resource_mut::<WorldData>()
            .chunks
            .get_mut(&IVec3::ZERO)
            .unwrap()
            .set(IVec3::new(1, 1, 2), STONE);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 1.0);
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .x = 1.5;
        mining_tick(&mut app, 0.25);
        let target = app.world().resource::<MiningState>().target.unwrap();
        assert_eq!(target.position, IVec3::new(1, 1, 2));
        assert!((target.elapsed - 0.25).abs() < 0.001);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        mining_tick(&mut app, 0.1);
        assert!(app.world().resource::<MiningState>().target.is_none());
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 0.25);
        app.world_mut().resource_mut::<CraftingUi>().inventory_open = true;
        mining_tick(&mut app, 0.25);
        assert!(app.world().resource::<MiningState>().target.is_none());
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            block_at(app.world().resource::<WorldData>(), IVec3::new(1, 1, 2)),
            STONE
        );
    }

    #[test]
    fn holding_attack_does_not_repeat_mob_hits_or_mine_through_mobs() {
        let tools = CraftedTools {
            sword_durability: 3,
            ..default()
        };
        let (mut app, _, _) = mining_test_app(STONE, tools);
        let mob = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.5, 0.7, 3.0),
                Mob {
                    kind: MobKind::Grazer,
                    health: 6,
                    attack_elapsed: 0.0,
                    wander_elapsed: 0.0,
                    wander_angle: 0.0,
                    wander_step: 0,
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 0.25);
        mining_tick(&mut app, 3.0);
        assert_eq!(app.world().get::<Mob>(mob).unwrap().health, 4);
        assert!(matches!(
            app.world().resource::<Inventory>().slots[0],
            Some(ItemStack {
                item: Item::Tool {
                    kind: ToolKind::Sword,
                    durability: 2
                },
                ..
            })
        ));
        assert!(app.world().resource::<MiningState>().target.is_none());
        assert_eq!(
            block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 2)),
            STONE
        );
    }

    #[test]
    fn right_click_cancels_mining_and_still_places_blocks() {
        let (mut app, _, overlay) = mining_test_app(STONE, CraftedTools::default());
        app.world_mut()
            .resource_mut::<Inventory>()
            .try_add(Item::Block(GRASS), 1)
            .unwrap();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(&mut app, 1.0);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        mining_tick(&mut app, 0.1);
        assert!(app.world().resource::<MiningState>().target.is_none());
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 2)),
            STONE
        );
        assert_eq!(
            block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 3)),
            GRASS
        );
        assert_eq!(app.world().resource::<Inventory>().count(GRASS), 0);
    }

    #[test]
    fn pausing_clears_mining_progress_and_hides_overlay() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<MiningState>()
            .add_systems(OnExit(AppState::InGame), reset_mining);
        let overlay = app
            .world_mut()
            .spawn((Visibility::Visible, MiningOverlay))
            .id();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        app.update();
        app.world_mut().resource_mut::<MiningState>().target = Some(MiningTarget {
            position: IVec3::ZERO,
            block: STONE,
            tool: None,
            elapsed: 1.0,
            duration: 3.0,
        });
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.update();
        assert!(app.world().resource::<MiningState>().target.is_none());
        assert_eq!(
            *app.world().get::<Visibility>(overlay).unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn mining_cracks_increase_with_each_stage() {
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let stages = create_mining_materials(&mut images, &mut materials);
        assert_eq!(stages.len(), MINING_CRACK_STAGES);
        let mut previous_count = 0;
        for handle in stages {
            let material = materials.get(&handle).unwrap();
            let image = images
                .get(material.base_color_texture.as_ref().unwrap())
                .unwrap();
            assert_eq!(image.texture_descriptor.size.width, 32);
            assert_eq!(image.texture_descriptor.size.height, 32);
            let count = image
                .data
                .chunks_exact(4)
                .filter(|pixel| pixel[3] != 0)
                .count();
            assert!(count > previous_count);
            assert!(count < 32 * 32);
            previous_count = count;
        }
    }

    #[test]
    fn loading_creates_player_and_textured_chunks_before_saving() {
        let path = temporary_world_path();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin))
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_state::<AppState>()
            .init_resource::<TextureAssets>()
            .init_resource::<MobAssets>()
            .init_resource::<PendingWorld>()
            .init_resource::<LoadingProgress>()
            .init_resource::<WorldData>()
            .init_resource::<Inventory>()
            .init_resource::<Survival>()
            .init_resource::<CraftedTools>()
            .init_resource::<MiningState>()
            .init_resource::<MiningVisuals>()
            .init_resource::<SelectedBlock>()
            .init_resource::<SelectedHotbarSlot>()
            .init_resource::<DayNightCycle>()
            .insert_resource(WorldStorage {
                path: Some(path.clone()),
                ..default()
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(WorldScenePlugin)
            .add_systems(
                Update,
                loading_world_step.run_if(in_state(AppState::LoadingWorld)),
            )
            .add_systems(
                Update,
                pause_menu_actions.run_if(in_state(AppState::Paused)),
            );

        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::LoadingWorld);
        app.update();
        let atlas = app
            .world()
            .resource::<TextureAssets>()
            .atlas
            .clone()
            .expect("atlas created");
        assert!(app
            .world()
            .resource::<Assets<Image>>()
            .get(&atlas)
            .is_some());

        {
            let mut world = app.world_mut().resource_mut::<WorldData>();
            world.name = "Prueba".into();
            world.chunks.insert(IVec3::ZERO, Chunk::new());
        }
        {
            let mut progress = app.world_mut().resource_mut::<LoadingProgress>();
            progress.initialized = true;
            progress.stage = LoadingStage::Meshes;
            progress.coordinates.push(IVec3::ZERO);
            progress.total = 1;
        }
        app.update();
        let material = app
            .world()
            .resource::<LoadingProgress>()
            .material
            .clone()
            .unwrap();
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .unwrap()
                .base_color_texture,
            Some(atlas)
        );
        app.update();
        let player = app
            .world_mut()
            .query_filtered::<Entity, With<Player>>()
            .single(app.world());
        let position = app.world().get::<Transform>(player).unwrap().translation;

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Player>>()
                .single(app.world()),
            player
        );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.update();
        app.world_mut()
            .spawn((Interaction::Pressed, MenuAction(MenuCommand::SaveAndMenu)));
        app.update();
        let save = load_world(&path).expect("player and world saved");
        assert_eq!(save.player_position, position.to_array());
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::MainMenu
        );
        fs::remove_file(&path).expect("remove temporary world file");
        fs::remove_dir(path.parent().unwrap()).expect("remove temporary world directory");
    }

    #[test]
    fn generated_atlas_contains_each_block_texture_and_grass_top() {
        use bevy::image::{CompressedImageFormats, ImageType};

        let atlas = include_bytes!(concat!(env!("OUT_DIR"), "/block_atlas.rgba"));
        let size = texture_layout::ATLAS_SIZE as usize;
        let tile_size = texture_layout::TILE_SIZE as usize;
        assert_eq!(atlas.len(), size * size * 4);
        let mut tiles = HashSet::new();
        for &(tile, filename) in texture_layout::TEXTURE_FILES {
            assert!(tiles.insert(tile), "duplicate tile {tile}");
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/blocks")
                .join(filename);
            let bytes = fs::read(&path).expect("read source texture");
            let image = Image::from_buffer(
                &bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::nearest(),
                RenderAssetUsages::MAIN_WORLD,
            )
            .expect("decode source texture");
            assert_eq!(image.texture_descriptor.size.width as usize, tile_size);
            assert_eq!(image.texture_descriptor.size.height as usize, tile_size);
            let x = tile % texture_layout::ATLAS_COLUMNS as usize * tile_size;
            let y = tile / texture_layout::ATLAS_COLUMNS as usize * tile_size;
            for row in 0..tile_size {
                let offset = ((y + row) * size + x) * 4;
                let source = row * tile_size * 4;
                assert_eq!(
                    &atlas[offset..offset + tile_size * 4],
                    &image.data[source..source + tile_size * 4],
                    "{filename}, row {row}"
                );
            }
        }
        for tile in 1..BLOCK_COUNT {
            assert!(tiles.contains(&tile), "missing block tile {tile}");
        }
        let (grass_top, _, _, _) = rendering::block_uv(GRASS, 2, 0);
        let (grass_side, _, _, _) = rendering::block_uv(GRASS, 0, 0);
        let top_tile = texture_layout::texture_index("grass_top");
        let side_tile = texture_layout::texture_index("grass_side");
        assert_eq!(
            grass_top,
            [
                (top_tile % texture_layout::ATLAS_COLUMNS as usize) as f32 / 16.0,
                (top_tile / texture_layout::ATLAS_COLUMNS as usize) as f32 / 16.0,
            ]
        );
        assert_eq!(
            grass_side,
            [
                (side_tile % texture_layout::ATLAS_COLUMNS as usize) as f32 / 16.0,
                (side_tile / texture_layout::ATLAS_COLUMNS as usize + 1) as f32 / 16.0,
            ]
        );
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
                let height = terrain_height(x, z, 42);
                assert!((8..=WORLD_HEIGHT - 16).contains(&height), "height {height}");
            }
        }
    }

    #[test]
    fn new_worlds_generate_connected_vertical_terrain_and_preserve_legacy_worlds() {
        let seed = 42;
        let mut found_cave = false;
        let mut found_water = false;
        'search: for z in -64..64 {
            for x in -64..64 {
                let height = terrain_height(x, z, seed);
                if !found_cave && height > SEA_LEVEL + 8 {
                    for y in SEA_LEVEL + 1..height - 4 {
                        if !is_cave(x, y, z, seed) {
                            continue;
                        }
                        let coord = IVec3::new(
                            x.div_euclid(CHUNK_SIZE),
                            y.div_euclid(CHUNK_SIZE),
                            z.div_euclid(CHUNK_SIZE),
                        );
                        let local = IVec3::new(
                            x.rem_euclid(CHUNK_SIZE),
                            y.rem_euclid(CHUNK_SIZE),
                            z.rem_euclid(CHUNK_SIZE),
                        );
                        assert_eq!(generate_chunk(coord, seed).get(local), AIR);
                        found_cave = true;
                        break;
                    }
                }

                if !found_water && height < SEA_LEVEL - 1 {
                    let coord = IVec3::new(
                        x.div_euclid(CHUNK_SIZE),
                        SEA_LEVEL.div_euclid(CHUNK_SIZE),
                        z.div_euclid(CHUNK_SIZE),
                    );
                    let local = IVec3::new(
                        x.rem_euclid(CHUNK_SIZE),
                        SEA_LEVEL.rem_euclid(CHUNK_SIZE),
                        z.rem_euclid(CHUNK_SIZE),
                    );
                    assert_eq!(generate_chunk(coord, seed).get(local), WATER);
                    found_water = true;
                }
                if found_cave && found_water {
                    break 'search;
                }
            }
        }
        assert!(found_cave, "seeded terrain should contain a dry cave");
        assert!(found_water, "low terrain should fill to sea level");

        let legacy = WorldData {
            seed,
            terrain_version: 1,
            ..default()
        };
        let legacy_surface = chunk_for_coordinate(&legacy, IVec3::ZERO);
        assert_eq!(
            legacy_surface.blocks,
            generate_legacy_chunk(IVec3::ZERO, seed).blocks
        );
        assert!(chunk_for_coordinate(&legacy, IVec3::Y)
            .blocks
            .iter()
            .all(|block| *block == AIR));
    }

    #[test]
    fn generated_chunks_include_varied_terrain_and_new_blocks() {
        let mut biomes = HashSet::new();
        let mut present = HashSet::new();
        let mut found_mountain = false;
        let mut found_desert = false;
        let deterministic_chunk = generate_chunk(IVec3::new(-4, 3, -4), 42);
        let mut generated_chunks = HashMap::new();

        for chunk_z in -4..=4 {
            for chunk_x in -4..=4 {
                for chunk_y in 0..WORLD_HEIGHT / CHUNK_SIZE {
                    let coord = IVec3::new(chunk_x, chunk_y, chunk_z);
                    let chunk = generate_chunk(coord, 42);
                    present.extend(chunk.blocks.iter().copied());
                    generated_chunks.insert(coord, chunk);
                }
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let world_x = chunk_x * CHUNK_SIZE + x;
                        let world_z = chunk_z * CHUNK_SIZE + z;
                        let biome = biome_at(world_x, world_z, 42);
                        let height = terrain_height(world_x, world_z, 42);
                        let coord = IVec3::new(chunk_x, height.div_euclid(CHUNK_SIZE), chunk_z);
                        let local = IVec3::new(x, height.rem_euclid(CHUNK_SIZE), z);
                        assert_eq!(
                            generated_chunks[&coord].get(local),
                            biome.surface_block(),
                            "surface at ({world_x}, {height}, {world_z})"
                        );
                        biomes.insert(biome);
                        found_mountain |= biome == Biome::Mountain && height >= 80;
                        found_desert |= biome == Biome::Desert;
                    }
                }
            }
        }

        assert!(
            biomes.contains(&Biome::Plains),
            "generated biomes: {biomes:?}"
        );
        assert!(biomes.contains(&Biome::Forest));
        assert!(
            biomes.contains(&Biome::Desert),
            "generated biomes: {biomes:?}"
        );
        assert!(biomes.contains(&Biome::Mountain));
        assert!(found_mountain);
        assert!(found_desert);
        assert!(present.contains(&GRASS));
        assert!(present.contains(&DIRT));
        assert!(present.contains(&STONE));
        assert!(present.contains(&SAND));
        assert!(present.contains(&WOOD));
        assert!(present.contains(&LEAVES));
        assert!(present.contains(&WATER));
        assert!(present.iter().all(|block| *block < BLOCK_COUNT as u8));

        // Test that new block types are properly defined
        assert_ne!(ICE, AIR);
        assert_ne!(SNOW, AIR);
        assert_ne!(MUD, AIR);
        assert_ne!(OBSIDIAN, AIR);
        assert_ne!(GRAVEL, AIR);

        assert_eq!(
            generate_chunk(IVec3::new(-4, 3, -4), 42).blocks,
            deterministic_chunk.blocks
        );
    }

    #[test]
    fn streaming_coordinates_cover_the_radius_around_negative_centers() {
        let center = IVec3::new(-2, 0, 3);
        let coordinates =
            chunk_coordinates_around(center, STREAM_LOAD_RADIUS, STREAM_VERTICAL_RADIUS);

        assert_eq!(coordinates.len(), 98);
        assert!(coordinates.contains(&center));
        assert!(coordinates.contains(&IVec3::new(-5, 0, 0)));
        assert!(coordinates.contains(&IVec3::new(1, 0, 6)));
        assert!(coordinates
            .iter()
            .all(|coordinate| (0..=1).contains(&coordinate.y)));
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
    fn adjacent_blocks_keep_separate_faces_and_texture_variants() {
        let mut chunk = Chunk::new();
        chunk.set(IVec3::ZERO, STONE);
        chunk.set(IVec3::X, STONE);
        let world = WorldData {
            chunks: HashMap::from([(IVec3::ZERO, chunk)]),
            ..default()
        };
        let mesh = rendering::build_chunk_mesh(&world, IVec3::ZERO);
        let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("block mesh positions must be 3D floats");
        };
        let bevy::render::mesh::VertexAttributeValues::Float32x2(uvs) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!("block mesh UVs must be 2D floats");
        };

        let mut top_faces = positions
            .chunks_exact(4)
            .zip(uvs.chunks_exact(4))
            .filter_map(|(vertices, face_uvs)| {
                vertices
                    .iter()
                    .all(|position| position[1] == 1.0)
                    .then_some((vertices, face_uvs))
            })
            .collect::<Vec<_>>();
        top_faces.sort_by(|(left, _), (right, _)| {
            left.iter()
                .map(|position| position[0])
                .fold(f32::INFINITY, f32::min)
                .total_cmp(
                    &right
                        .iter()
                        .map(|position| position[0])
                        .fold(f32::INFINITY, f32::min),
                )
        });

        assert_eq!(top_faces.len(), 2);
        for (x, (positions, uvs)) in top_faces.into_iter().enumerate() {
            let min_x = positions
                .iter()
                .map(|position| position[0])
                .fold(f32::INFINITY, f32::min);
            let max_x = positions
                .iter()
                .map(|position| position[0])
                .fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(max_x - min_x, 1.0);
            assert_eq!(min_x, x as f32);

            let variant = rendering::block_variant(x as i32, 0, 0);
            let (uv0, uv1, uv2, uv3) = rendering::block_uv(STONE, 2, variant);
            assert_eq!(uvs, [uv0, uv1, uv2, uv3]);
        }
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
    fn player_recovery_only_moves_out_of_actual_solid_collisions() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::new(0, 0, 0), STONE);
        chunk.set(IVec3::new(1, 1, 0), STONE);
        chunk.set(IVec3::new(1, 2, 0), STONE);
        chunk.set(IVec3::new(2, 1, 0), SLAB);
        world.chunks.insert(IVec3::ZERO, chunk);

        let standing = Vec3::new(0.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
        assert_eq!(find_safe_spawn_position(&world, standing), standing);
        let above_ground = standing + Vec3::Y * 0.5;
        assert_eq!(find_safe_spawn_position(&world, above_ground), above_ground);
        let in_slab = Vec3::new(2.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
        assert_eq!(find_safe_spawn_position(&world, in_slab), in_slab);

        let embedded = Vec3::new(0.75, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
        let recovered = find_safe_spawn_position(&world, embedded);
        assert!(!collides_with_world(&world, recovered));
        assert!((recovered.y - (3.0 + PLAYER_EYE_HEIGHT)).abs() < 0.0001);
        assert_eq!(recovered.xz(), embedded.xz());
    }

    #[test]
    fn stationary_player_stays_on_ground_and_can_jump() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::ZERO, STONE);
        world.chunks.insert(IVec3::ZERO, chunk);

        let mut app = App::new();
        let mut time = Time::<Fixed>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.insert_resource(time)
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(world)
            .insert_resource(Survival::default())
            .add_systems(Update, move_player);
        let mut window = Window::default();
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        app.world_mut().spawn((window, PrimaryWindow));
        let start = Vec3::new(0.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(start),
                Player {
                    yaw: 0.0,
                    pitch: 0.0,
                    velocity: Vec3::ZERO,
                    grounded: true,
                    flying: false,
                    jump_buffer: 0.0,
                    coyote_time: 0.0,
                    step_up_remaining: 0.0,
                    jump_key_down: false,
                    flight_key_down: false,
                },
            ))
            .id();

        for _ in 0..120 {
            app.update();
            let position = app.world().get::<Transform>(entity).unwrap().translation;
            assert!((position.y - start.y).abs() < 0.001, "{position:?}");
            assert!(app.world().get::<Player>(entity).unwrap().grounded);
        }

        app.world_mut()
            .get_mut::<Transform>(entity)
            .unwrap()
            .translation
            .y += 2.0;
        app.world_mut().get_mut::<Player>(entity).unwrap().grounded = false;
        for _ in 0..240 {
            app.update();
        }
        let landed = app.world().get::<Transform>(entity).unwrap().translation;
        assert!((landed.y - start.y).abs() < 0.001, "{landed:?}");
        assert!(app.world().get::<Player>(entity).unwrap().grounded);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert!(app.world().get::<Transform>(entity).unwrap().translation.y > start.y);
    }

    #[test]
    fn flying_inside_blocks_is_not_interrupted_by_recovery() {
        let mut world = WorldData::default();
        let mut chunk = Chunk::new();
        chunk.set(IVec3::ZERO, STONE);
        world.chunks.insert(IVec3::ZERO, chunk);

        let mut app = App::new();
        let mut time = Time::<Fixed>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.insert_resource(time)
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(world)
            .insert_resource(Survival::default())
            .add_systems(Update, move_player);
        let mut window = Window::default();
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        app.world_mut().spawn((window, PrimaryWindow));
        let start = Vec3::new(0.5, 1.0, 0.5);
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(start),
                Player {
                    yaw: 0.0,
                    pitch: 0.0,
                    velocity: Vec3::ZERO,
                    grounded: false,
                    flying: true,
                    jump_buffer: 0.0,
                    coyote_time: 0.0,
                    step_up_remaining: 0.0,
                    jump_key_down: false,
                    flight_key_down: false,
                },
            ))
            .id();

        app.update();
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().translation,
            start
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        let position = app.world().get::<Transform>(entity).unwrap().translation;
        assert!(!collides_with_world(
            app.world().resource::<WorldData>(),
            position
        ));
        assert!((position.y - (1.0 + PLAYER_EYE_HEIGHT)).abs() < 0.001);
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
            hunger: 16.0, // Changed from 15 to 16, so 16 + 4 = 20 = MAX_HUNGER
            food: 2,
            ..default()
        };

        assert!(eat_food(&mut survival));
        assert_eq!(survival.hunger, MAX_HUNGER);
        assert_eq!(survival.food, 1);
        assert!(!eat_food(&mut survival));
        survival.hunger = 0.0;
        assert!(eat_food(&mut survival));
        assert_eq!(survival.hunger, 4.0); // BerryRation provides 4 saturation, not 6
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
    fn equipped_tools_only_help_with_matching_blocks_and_break() {
        let mut inventory = Inventory::default();
        inventory
            .try_add(
                Item::Tool {
                    kind: ToolKind::Pickaxe,
                    durability: 1,
                },
                1,
            )
            .unwrap();
        assert_eq!(
            selected_mining_tool(STONE, inventory.slots[0]),
            Some(MiningTool::Pickaxe)
        );
        assert_eq!(selected_mining_tool(WOOD, inventory.slots[0]), None);
        assert_eq!(wear_selected_tool(&mut inventory, 0), Some(0));
        assert_eq!(selected_mining_tool(STONE, inventory.slots[0]), None);
    }

    #[test]
    fn shovel_is_twice_as_efficient_on_sand_and_dirt() {
        let shovel = Some(ItemStack {
            item: Item::Tool {
                kind: ToolKind::Shovel,
                durability: 10,
            },
            count: 1,
        });
        assert_eq!(selected_mining_tool(SAND, shovel), Some(MiningTool::Shovel));
        assert_eq!(selected_mining_tool(DIRT, shovel), Some(MiningTool::Shovel));
        assert_eq!(selected_mining_tool(STONE, shovel), None);
        assert_eq!(
            mining_duration(DIRT, selected_mining_tool(DIRT, shovel)),
            mining_duration(DIRT, None) / 2.0
        );
    }

    #[test]
    fn sword_consumes_durability_on_mob_hits() {
        let mut inventory = Inventory::default();
        inventory
            .try_add(
                Item::Tool {
                    kind: ToolKind::Sword,
                    durability: 5,
                },
                1,
            )
            .unwrap();
        for remaining in (0..5).rev() {
            assert_eq!(wear_selected_tool(&mut inventory, 0), Some(remaining));
        }
        assert!(inventory.slots[0].is_none());
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
            .insert_resource(SelectedHotbarSlot::default())
            .insert_resource(Inventory::default())
            .insert_resource(CraftedTools::default())
            .insert_resource(MiningState::default())
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
                let center_y = terrain_height(
                    x * CHUNK_SIZE + CHUNK_SIZE / 2,
                    z * CHUNK_SIZE + CHUNK_SIZE / 2,
                    seed,
                )
                .div_euclid(CHUNK_SIZE);
                for y in (center_y - 1).max(0)..=(center_y + 1).min(WORLD_HEIGHT / CHUNK_SIZE - 1) {
                    let coordinate = IVec3::new(x, y, z);
                    world
                        .chunks
                        .insert(coordinate, generate_chunk(coordinate, seed));
                }
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
        inventory.try_add(Item::Block(DIRT), 17).unwrap();
        inventory.try_add(Item::Block(SAND), 5).unwrap();
        inventory.try_add(Item::Block(WOOD), 3).unwrap();
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
            jump_buffer: 0.0,
            coyote_time: 0.0,
            step_up_remaining: 0.0,
            jump_key_down: false,
            flight_key_down: false,
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
            terrain_version: TERRAIN_VERSION,
            time_of_day_seconds: 43_200.0,
            player_position: [4.5, 9.0, -2.0],
            player_yaw: 1.2,
            player_pitch: -0.4,
            selected_block: DIRT,
            selected_hotbar_slot: 1,
            inventory: [0, 0, 17, 0],
            extra_inventory: vec![0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
            slot_inventory: Some(Inventory::default()),
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

        assert_eq!(save.terrain_version, 1);
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
        assert!(validate_world_save(&save).is_err());
    }

    #[test]
    fn hotbar_has_nine_slots_and_scroll_wraps_in_both_directions() {
        assert_eq!(HOTBAR_SLOT_COUNT, 9);
        assert_eq!(cycle_hotbar_selection(0, -1), 8);
        assert_eq!(cycle_hotbar_selection(8, 1), 0);
        assert_eq!(cycle_hotbar_selection(2, 3), 5);
        assert_eq!(mouse_wheel_steps(1.0, MouseScrollUnit::Line), -1);
        assert_eq!(mouse_wheel_steps(-80.0, MouseScrollUnit::Pixel), 2);
        assert_eq!(mouse_wheel_steps(0.0, MouseScrollUnit::Line), 0);
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
            terrain_version: TERRAIN_VERSION,
            time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
            player_position: [0.0, 0.0, 0.0],
            player_yaw: 0.0,
            player_pitch: 0.0,
            selected_block: GRASS,
            selected_hotbar_slot: 0,
            inventory: [0; LEGACY_BLOCK_COUNT],
            extra_inventory: vec![0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
            slot_inventory: Some(Inventory::default()),
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
        let mut world = WorldData {
            seed: 42,
            ..default()
        };
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
        let mut world = WorldData {
            seed: 42,
            ..default()
        };
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
        assert!(!is_block_solid(&world, stair_pos), "Stairs allow movement");
        assert!(!is_block_solid(&world, slab_pos), "Slabs allow movement");

        // But they render in the world (chunk has the blocks)
        let chunk = world.chunks.get(&IVec3::ZERO).unwrap();
        assert_eq!(chunk.get(stair_pos), STAIRS);
        assert_eq!(chunk.get(slab_pos), SLAB);
    }
}
