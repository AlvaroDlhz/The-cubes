//! World save format, migration, validation and chunk persistence.

use crate::*;

pub(crate) const SAVE_VERSION: u32 = 3;

pub(crate) const AUTOSAVE_INTERVAL_SECONDS: f32 = 60.0;

#[derive(Serialize, Deserialize)]
pub(crate) struct WorldSave {
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) seed: u64,
    #[serde(default = "legacy_terrain_version")]
    pub(crate) terrain_version: u32,
    #[serde(default = "default_time_of_day_seconds")]
    pub(crate) time_of_day_seconds: f32,
    pub(crate) player_position: [f32; 3],
    #[serde(default)]
    pub(crate) player_yaw: f32,
    #[serde(default)]
    pub(crate) player_pitch: f32,
    pub(crate) selected_block: u8,
    #[serde(default)]
    pub(crate) selected_hotbar_slot: usize,
    pub(crate) inventory: [u32; LEGACY_BLOCK_COUNT],
    #[serde(default, deserialize_with = "deserialize_inventory_counts")]
    pub(crate) extra_inventory: Vec<u32>,
    #[serde(default)]
    pub(crate) slot_inventory: Option<Inventory>,
    #[serde(default = "default_health")]
    pub(crate) health: f32,
    #[serde(default = "default_hunger")]
    pub(crate) hunger: f32,
    #[serde(default)]
    pub(crate) experience: u32,
    #[serde(default = "default_starting_food")]
    pub(crate) food: u32,
    #[serde(default)]
    pub(crate) pickaxe_durability: u32,
    #[serde(default)]
    pub(crate) axe_durability: u32,
    #[serde(default)]
    pub(crate) shovel_durability: u32,
    #[serde(default)]
    pub(crate) sword_durability: u32,
    pub(crate) chunks: Vec<ChunkSave>,
    #[serde(default)]
    pub(crate) furnaces: Vec<FurnaceSave>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FurnaceSave {
    pub(crate) position: [i32; 3],
    pub(crate) furnace: Furnace,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChunkSave {
    pub(crate) coord: [i32; 3],
    pub(crate) blocks: Vec<u8>,
}

pub(crate) fn legacy_terrain_version() -> u32 {
    1
}

pub(crate) fn deserialize_inventory_counts<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
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

#[derive(Resource, Default)]
pub(crate) struct WorldStorage {
    pub(crate) path: Option<PathBuf>,
    pub(crate) legacy_chunks: HashMap<IVec3, Chunk>,
    pub(crate) dirty_chunks: HashSet<IVec3>,
    pub(crate) failed_chunks: HashSet<IVec3>,
    pub(crate) autosave_elapsed: f32,
    pub(crate) retry_elapsed: f32,
    pub(crate) retry_generation: u64,
    pub(crate) metadata_save_requested: bool,
    pub(crate) status: String,
}

pub(crate) fn data_directory() -> Result<PathBuf, String> {
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

pub(crate) fn default_time_of_day_seconds() -> f32 {
    DEFAULT_TIME_OF_DAY_SECONDS
}

pub(crate) fn default_health() -> f32 {
    MAX_HEALTH
}

pub(crate) fn default_hunger() -> f32 {
    MAX_HUNGER
}

pub(crate) fn default_starting_food() -> u32 {
    3
}

pub(crate) fn world_file_path(name: &str) -> Result<PathBuf, String> {
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

pub(crate) fn list_saved_worlds() -> Result<Vec<(String, PathBuf)>, String> {
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

pub(crate) fn load_world(path: &Path) -> Result<WorldSave, String> {
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

pub(crate) fn migrate_world_save(save: &mut WorldSave) -> Result<(), String> {
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

pub(crate) fn validate_world_save(save: &WorldSave) -> Result<(), String> {
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
    if save.furnaces.len() > 4096 {
        return Err("El mundo contiene demasiados hornos.".into());
    }
    let mut furnace_positions = HashSet::new();
    for saved in &save.furnaces {
        if !saved.furnace.is_valid() || !furnace_positions.insert(saved.position) {
            return Err("Un horno guardado tiene datos inválidos.".into());
        }
    }
    if save.inventory[0] != 0 {
        return Err("El inventario contiene aire como bloque.".into());
    }
    Ok(())
}

pub(crate) fn snapshot_world(
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
        furnaces: {
            let mut furnaces: Vec<_> = world
                .furnaces
                .iter()
                .map(|(position, furnace)| FurnaceSave {
                    position: position.to_array(),
                    furnace: furnace.clone(),
                })
                .collect();
            furnaces.sort_by_key(|saved| saved.position);
            furnaces
        },
    }
}

pub(crate) fn chunk_directory(world_path: &Path) -> PathBuf {
    world_path.with_extension("chunks")
}

pub(crate) fn chunk_file_path(world_path: &Path, coord: IVec3) -> PathBuf {
    chunk_directory(world_path).join(format!("{}_{}_{}.ron", coord.x, coord.y, coord.z))
}

pub(crate) fn save_chunk_file(
    world_path: &Path,
    coord: IVec3,
    chunk: &Chunk,
) -> Result<(), String> {
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

pub(crate) fn load_chunk_file(world_path: &Path, coord: IVec3) -> Result<Option<Chunk>, String> {
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

pub(crate) fn save_world(world_path: &Path, save: WorldSave) -> Result<(), String> {
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

pub(crate) fn save_all_chunks(
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

pub(crate) fn load_or_generate_chunk(
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

pub(crate) fn autosave_world(
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
