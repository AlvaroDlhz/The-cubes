//! Initial world loading, chunk streaming around the player and mesh rebuilds.

use crate::*;

pub(crate) const STREAM_LOAD_RADIUS: i32 = 10;

pub(crate) const STREAM_UNLOAD_RADIUS: i32 = 11;

pub(crate) const STARTUP_LOAD_RADIUS: i32 = 3;

pub(crate) const STREAM_VERTICAL_RADIUS: i32 = 1;

pub(crate) const MAX_DIRTY_CHUNK_MESH_REBUILDS_PER_FRAME: usize = 8;

pub(crate) const MAX_CHUNK_UNLOADS_PER_FRAME: usize = 2;

#[derive(Resource, Default)]
pub(crate) struct PendingWorld {
    pub(crate) save: Option<WorldSave>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) is_new_world: bool,
}

#[derive(Resource, Default)]
pub(crate) struct LoadingProgress {
    pub(crate) coordinates: Vec<IVec3>,
    pub(crate) completed: usize,
    pub(crate) total: usize,
    pub(crate) chunk_count: usize,
    pub(crate) chunk_cursor: usize,
    pub(crate) stage: LoadingStage,
    pub(crate) material: Option<Handle<StandardMaterial>>,
    pub(crate) is_new_world: bool,
    pub(crate) initialized: bool,
}

#[derive(Default, PartialEq, Eq)]
pub(crate) enum LoadingStage {
    #[default]
    Chunks,
    Meshes,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkStreamingState {
    pub(crate) center: Option<IVec3>,
    pub(crate) vertical_radius: i32,
    pub(crate) retry_generation: u64,
    pub(crate) pending_unloads: Vec<(Entity, IVec3)>,
    pub(crate) next_unload: usize,
    pub(crate) pending: Vec<IVec3>,
    pub(crate) next: usize,
    pub(crate) chunk_entities: HashMap<IVec3, Entity>,
}

impl ChunkStreamingState {
    pub(crate) fn invalidate(&mut self) {
        self.center = None;
        self.pending_unloads.clear();
        self.next_unload = 0;
        self.pending.clear();
        self.next = 0;
        self.chunk_entities.clear();
    }
}

#[derive(Component)]
pub(crate) struct ChunkMesh(pub(crate) IVec3);

pub(crate) fn begin_loading_world(
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
    world.door_states.clear();
    world.furnaces = save
        .furnaces
        .iter()
        .map(|saved| (IVec3::from_array(saved.position), saved.furnace.clone()))
        .collect();
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
    crafting_ui.furnace = None;
    crafting_ui.furnace_requested = None;
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

pub(crate) fn chunk_coordinates_around(
    center: IVec3,
    radius: i32,
    vertical_radius: i32,
) -> Vec<IVec3> {
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

pub(crate) fn loading_world_step(
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

pub(crate) fn stream_world_chunks(
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

pub(crate) fn mark_chunk_neighbors_dirty(world: &mut WorldData, coord: IVec3) {
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

pub(crate) fn rebuild_dirty_chunks(
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

pub(crate) fn mark_dirty(world: &mut WorldData, coord: IVec3, local: IVec3) {
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
