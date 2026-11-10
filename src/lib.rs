#![allow(clippy::too_many_arguments)]
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
pub mod blocks;
pub mod furnace;
pub mod item_inventory;
pub mod texture_layout;
pub mod world;

// Selective imports from modules
use blocks::*;
use furnace::Furnace;
use item_inventory::{Inventory, Item, ItemStack, ToolKind, HOTBAR_SIZE, INVENTORY_SIZE};
use world::*;

// ============================================================================
// MODULE: rendering
// ============================================================================
// Handles block visualization, mesh generation, and texture mapping
// Pure functions that don't require mutable state beyond input parameters
mod hud;
mod inventory_ui;
mod menus;
mod mining;
mod mobs;
mod player;
mod rendering;
mod save;
mod sky;
mod streaming;
mod survival;

use hud::*;
use inventory_ui::*;
use menus::*;
use mining::*;
use mobs::*;
use player::*;
// Named explicitly: Bevy's prelude also exports a `PointerState`.
use player::PointerState;
use save::*;
use sky::*;
use streaming::*;
use survival::*;

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

pub fn run() {
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
                        tick_furnaces,
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
struct UiCamera;

#[cfg(feature = "plains-audio")]
#[derive(Component)]
struct PlainsAmbience;

#[derive(Resource, Default)]
struct TextureAssets {
    atlas: Option<Handle<Image>>,
    water_material: Option<Handle<StandardMaterial>>,
    lava_material: Option<Handle<StandardMaterial>>,
}

#[derive(Resource, Default)]
struct CraftedTools {
    pickaxe_durability: u32,
    axe_durability: u32,
    shovel_durability: u32,
    sword_durability: u32,
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

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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
    world.door_states.clear();
    world.furnaces.clear();
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

#[derive(Component)]
struct GameEntity;

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

#[cfg(test)]
mod tests;
