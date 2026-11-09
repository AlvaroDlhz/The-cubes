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

const CHUNK_SIZE: i32 = 16;
const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
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
const STREAM_LOAD_RADIUS: i32 = 2;
const STREAM_UNLOAD_RADIUS: i32 = 3;
const GAME_DAY_SECONDS: f32 = 24.0 * 60.0 * 60.0;
const GAME_SECONDS_PER_REAL_SECOND: f32 = 60.0;
const DEFAULT_TIME_OF_DAY_SECONDS: f32 = 8.0 * 60.0 * 60.0;
const SKY_RADIUS: f32 = 180.0;
const CELESTIAL_RADIUS: f32 = 120.0;
const STAR_RADIUS: f32 = 145.0;
const HOTBAR_SLOT_COUNT: usize = 6;
const HOTBAR_BLOCKS: [u8; HOTBAR_SLOT_COUNT] = [GRASS, DIRT, STONE, AIR, AIR, AIR];

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
        .init_resource::<WorldForm>()
        .init_resource::<PendingWorld>()
        .init_resource::<LoadingProgress>()
        .init_resource::<DayNightCycle>()
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
                capture_pointer,
                look,
                move_player,
                advance_day_night,
                update_day_night_lighting,
                update_sky,
                stream_world_chunks,
                select_block,
                select_hotbar,
                update_hud,
                update_hotbar,
                edit_blocks,
                rebuild_dirty_chunks,
                pause_from_game,
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
    is_new_world: bool,
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
    inventory: [u32; 4],
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
}

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
    counts: [u32; 4],
}

impl Inventory {
    fn count(&self, block: u8) -> u32 {
        self.counts.get(block as usize).copied().unwrap_or(0)
    }

    fn add(&mut self, block: u8) -> Result<(), u8> {
        if !matches!(block, GRASS | DIRT | STONE) {
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
            for y in 0..=height {
                let block = if y == height {
                    GRASS
                } else if y >= height - 3 {
                    DIRT
                } else {
                    STONE
                };
                chunk.set(IVec3::new(x, y, z), block);
            }
        }
    }
    chunk
}

fn setup_game_scene(
    mut commands: Commands,
    pending: Res<PendingWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

fn setup_pause_menu(mut commands: Commands) {
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
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(1.0, 0.45, 0.4)),
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
    mut pending: ResMut<PendingWorld>,
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
    world.chunks.clear();
    world.modified_chunks.clear();
    world.dirty.clear();
    world.name.clear();
    inventory.counts = [0; 4];
    selected.0 = GRASS;
    selected_slot.0 = 0;
    pending.save = None;
    pending.is_new_world = false;
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
                    let block = HOTBAR_BLOCKS[slot_index];
                    let color = match block {
                        GRASS => Color::srgb(0.25, 0.62, 0.22),
                        DIRT => Color::srgb(0.43, 0.30, 0.18),
                        STONE => Color::srgb(0.48, 0.50, 0.53),
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
                pending.is_new_world = false;
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
                    inventory: [0; 4],
                    chunks: Vec::new(),
                });
                pending.is_new_world = true;
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
    mut selected: ResMut<SelectedBlock>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    mut pending: ResMut<PendingWorld>,
    mut next: ResMut<NextState<AppState>>,
) {
    world.chunks.clear();
    world.modified_chunks.clear();
    world.dirty.clear();
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
    inventory.counts = save.inventory;
    selected.0 = save.selected_block;
    selected_slot.0 = resolve_hotbar_slot(save.selected_block, save.selected_hotbar_slot);

    world.modified_chunks = std::mem::take(&mut save.chunks)
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
    mut pending: ResMut<PendingWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: Query<&mut Text, With<LoadingText>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !progress.initialized {
        return;
    }
    match progress.stage {
        LoadingStage::Chunks if progress.chunk_cursor < progress.chunk_count => {
            if let Some(coord) = progress.coordinates.pop() {
                let chunk = chunk_for_coordinate(&world, coord);
                world.chunks.insert(coord, chunk);
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
                        base_color: Color::WHITE,
                        ..default()
                    }));
                }
                let Some(material) = progress.material.clone() else {
                    error!("Could not create the chunk material");
                    progress.initialized = false;
                    next.set(AppState::MainMenu);
                    return;
                };
                let mesh = meshes.add(build_chunk_mesh(&world, coord));
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
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    day_night: Res<DayNightCycle>,
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
                match save_world(snapshot_world(
                    &world,
                    transform.translation,
                    player,
                    &inventory,
                    selected.0,
                    selected_slot.0,
                    day_night.time_of_day_seconds,
                )) {
                    Ok(()) => next.set(AppState::MainMenu),
                    Err(error) => {
                        error!("Could not save world: {error}");
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
    mut next: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
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
    if !matches!(save.selected_block, AIR | GRASS | DIRT | STONE)
        || save.selected_hotbar_slot >= HOTBAR_SLOT_COUNT
    {
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
        if chunk.blocks.len() != CHUNK_VOLUME || chunk.blocks.iter().any(|block| *block > STONE) {
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

fn snapshot_world(
    world: &WorldData,
    player_position: Vec3,
    player: &Player,
    inventory: &Inventory,
    selected_block: u8,
    selected_hotbar_slot: usize,
    time_of_day_seconds: f32,
) -> WorldSave {
    let mut chunks = world
        .modified_chunks
        .iter()
        .map(|(coord, chunk)| ChunkSave {
            coord: coord.to_array(),
            blocks: chunk.blocks.clone(),
        })
        .collect::<Vec<_>>();
    chunks.sort_by_key(|chunk| chunk.coord);
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
        inventory: inventory.counts,
        chunks,
    }
}

fn save_world(save: WorldSave) -> Result<(), String> {
    validate_world_save(&save)?;
    let path = world_file_path(&save.name)?;
    let directory = path
        .parent()
        .ok_or_else(|| "No se pudo determinar el directorio de mundos.".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("No se pudo crear {}: {error}", directory.display()))?;
    let contents = ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("No se pudo serializar el mundo: {error}"))?;
    let temporary_path = path.with_extension("ron.tmp");
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("No se pudo escribir el mundo: {error}"))?;
    fs::rename(&temporary_path, &path)
        .map_err(|error| format!("No se pudo finalizar el guardado: {error}"))?;
    Ok(())
}

fn capture_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
) {
    pointer.captured_this_frame = false;
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
            }
            player.velocity.y = 0.0;
        }
    }
}

fn stream_world_chunks(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut world: ResMut<WorldData>,
    mut loading: ResMut<LoadingProgress>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_entities: Query<(Entity, &ChunkMesh)>,
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

    let chunk = chunk_for_coordinate(&world, coord);
    world.chunks.insert(coord, chunk);
    mark_chunk_neighbors_dirty(&mut world, coord);

    if loading.material.is_none() {
        loading.material = Some(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        }));
    }
    let Some(material) = loading.material.clone() else {
        error!("Could not create the chunk material");
        world.chunks.remove(&coord);
        return;
    };
    let mesh = meshes.add(build_chunk_mesh(&world, coord));
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
                if block_at(world, IVec3::new(x, y, z)) != AIR {
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
    magnitude * y.signum() as isize
}

fn update_hud(
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    inventory: Res<Inventory>,
    day_night: Res<DayNightCycle>,
    player: Query<&Player>,
    mut status: Query<&mut Text, With<HudStatus>>,
) {
    let (Ok(player), Ok(mut status)) = (player.get_single(), status.get_single_mut()) else {
        return;
    };
    let mode = if player.flying { "VOLAR" } else { "CAMINAR" };
    let block = match selected.0 {
        AIR => "Vacío",
        GRASS => "Césped [1]",
        DIRT => "Tierra [2]",
        STONE => "Piedra [3]",
        _ => "Desconocido",
    };
    status.0 = format!(
        "{mode}  [F] alternar\nHora: {}\nEspacio {}: {block} ×{}\n[Rueda] cambiar  |  [ESC] pausa",
        format_time_of_day(day_night.time_of_day_seconds),
        selected_slot.0 + 1,
        inventory.count(selected.0)
    );
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
    mouse: Res<ButtonInput<MouseButton>>,
    camera: Query<&Transform, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
    selected: Res<SelectedBlock>,
    mut inventory: ResMut<Inventory>,
    mut world: ResMut<WorldData>,
) {
    if pointer.captured_this_frame {
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
    let Some((hit, previous)) = raycast(&world, transform.translation, direction, REACH) else {
        return;
    };
    let target = if remove { hit } else { previous };
    let (chunk_coord, local) = block_address(target);
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
        }
    }
    let updated_chunk = world.chunks[&chunk_coord].blocks.clone();
    world.modified_chunks.insert(
        chunk_coord,
        Chunk {
            blocks: updated_chunk,
        },
    );
    mark_dirty(&mut world, chunk_coord, local);
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
    let dirty = world.dirty.drain().collect::<HashSet<_>>();
    for (chunk, mesh_handle) in &mut chunk_entities {
        if dirty.contains(&chunk.0) {
            let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
                error!("Missing mesh asset for chunk {:?}", chunk.0);
                continue;
            };
            *mesh = build_chunk_mesh(&world, chunk.0);
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

fn raycast(world: &WorldData, origin: Vec3, direction: Vec3, reach: f32) -> Option<(IVec3, IVec3)> {
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

    for _ in 0..256 {
        if block_at(world, cell) != AIR {
            return Some((cell, previous));
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

fn terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    let phase_x = (seed as u32 % 10_000) as f32 * 0.0007;
    let phase_z = ((seed >> 32) as u32 % 10_000) as f32 * 0.0009;
    let x = x as f32 * 0.14 + phase_x;
    let z = z as f32 * 0.14 + phase_z;
    (5.0 + x.sin() * 2.5 + z.cos() * 2.5)
        .round()
        .clamp(1.0, 12.0) as i32
}

fn greedy_rectangles(mask: &mut [u8], mut emit: impl FnMut(usize, usize, usize, usize, u8)) {
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

fn build_chunk_mesh(world: &WorldData, coord: IVec3) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut colors = Vec::<[f32; 4]>::new();
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
                        if block_at(world, neighbor) == AIR {
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
                    let base = positions.len() as u32;
                    for corner in corners {
                        positions.push(corner.to_array());
                        normals.push(normal.as_vec3().to_array());
                        colors.push(block_color(block, face_index));
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
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn block_color(block: u8, face: usize) -> [f32; 4] {
    let (r, g, b) = match (block, face) {
        (GRASS, 2) => (0.25, 0.68, 0.24),
        (GRASS, _) => (0.42, 0.31, 0.18),
        (DIRT, _) => (0.42, 0.31, 0.18),
        (STONE, _) => (0.48, 0.50, 0.52),
        _ => (1.0, 0.0, 1.0),
    };
    let shade = if face == 3 {
        0.72
    } else if face == 0 || face == 1 {
        0.86
    } else {
        1.0
    };
    [r * shade, g * shade, b * shade, 1.0]
}

fn player_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn streaming_coordinates_cover_the_radius_around_negative_centers() {
        let center = IVec3::new(-2, 0, 3);
        let coordinates = chunk_coordinates_around(center, STREAM_LOAD_RADIUS);

        assert_eq!(coordinates.len(), 25);
        assert!(coordinates.contains(&center));
        assert!(coordinates.contains(&IVec3::new(-4, 0, 1)));
        assert!(coordinates.contains(&IVec3::new(0, 0, 5)));
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

        greedy_rectangles(&mut mask, |u, v, width, height, block| {
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

        greedy_rectangles(&mut mask, |u, v, width, height, block| {
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
    fn blocks_cannot_be_placed_inside_the_player() {
        let eye_position = Vec3::new(0.5, 2.62, 0.5);

        assert!(block_overlaps_player(IVec3::new(0, 1, 0), eye_position));
        assert!(!block_overlaps_player(IVec3::new(0, 0, 0), eye_position));
        assert!(!block_overlaps_player(IVec3::new(2, 1, 0), eye_position));
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
        assert_eq!(loaded.chunks.len(), 1);
        assert_eq!(
            loaded.chunks[0].blocks[block_index(IVec3::new(1, 2, 3))],
            STONE
        );
    }

    #[test]
    fn world_save_defaults_new_fields_for_existing_saves() {
        let contents = r#"(version:1,name:"Prueba",seed:42,player_position:(0.0,0.0,0.0),selected_block:1,inventory:(0,0,0,0),chunks:[])"#;
        let save: WorldSave = ron::from_str(contents).expect("deserialize existing world save");

        assert_eq!(save.player_yaw, 0.0);
        assert_eq!(save.player_pitch, 0.0);
        assert_eq!(save.time_of_day_seconds, DEFAULT_TIME_OF_DAY_SECONDS);
        assert_eq!(save.selected_hotbar_slot, 0);
        validate_world_save(&save).expect("validate existing world save");
    }

    #[test]
    fn hotbar_has_six_slots_and_scroll_wraps_in_both_directions() {
        assert_eq!(HOTBAR_BLOCKS.len(), 6);
        assert_eq!(cycle_hotbar_selection(0, -1), 5);
        assert_eq!(cycle_hotbar_selection(5, 1), 0);
        assert_eq!(cycle_hotbar_selection(2, 3), 5);
        assert_eq!(resolve_hotbar_slot(DIRT, 0), 1);
        assert_eq!(resolve_hotbar_slot(AIR, 4), 4);
        assert_eq!(mouse_wheel_steps(1.0, MouseScrollUnit::Line), 1);
        assert_eq!(mouse_wheel_steps(-80.0, MouseScrollUnit::Pixel), -2);
        assert_eq!(mouse_wheel_steps(0.0, MouseScrollUnit::Line), 0);

        let mut selected = SelectedBlock(GRASS);
        let mut selected_slot = SelectedHotbarSlot::default();
        set_hotbar_selection(5, &mut selected, &mut selected_slot);
        assert_eq!(selected_slot.0, 5);
        assert_eq!(selected.0, AIR);
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
            inventory: [0; 4],
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
}
