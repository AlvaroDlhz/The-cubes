//! Loaded world state, block access, spawn point search and versioned terrain lookup.

use crate::*;

pub(crate) const WORLD_HEIGHT: i32 = 128;

pub(crate) const SEA_LEVEL: i32 = 48;

pub(crate) const TERRAIN_VERSION: u32 = 4;

#[derive(Resource)]
pub(crate) struct WorldData {
    pub(crate) chunks: HashMap<IVec3, Chunk>,
    pub(crate) modified_chunks: HashMap<IVec3, Chunk>,
    pub(crate) dirty: HashSet<IVec3>,
    pub(crate) name: String,
    pub(crate) seed: u64,
    pub(crate) terrain_version: u32,
    pub(crate) door_states: HashMap<IVec3, DoorState>,
    /// Contents and fire of every placed furnace, keyed by block position.
    pub(crate) furnaces: HashMap<IVec3, Furnace>,
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
            furnaces: HashMap::new(),
        }
    }
}

// Block state for doors (open/closed)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DoorState {
    Open,
    Closed,
}

pub(crate) fn chunk_for_coordinate(world: &WorldData, coord: IVec3) -> Chunk {
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

pub(crate) fn block_at(world: &WorldData, position: IVec3) -> u8 {
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

pub(crate) fn is_block_solid(world: &WorldData, position: IVec3) -> bool {
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

pub(crate) fn block_address(position: IVec3) -> (IVec3, IVec3) {
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

pub(crate) fn world_biome_at(world: &WorldData, x: i32, z: i32) -> Biome {
    if world.terrain_version >= TERRAIN_VERSION {
        biome_at(x, z, world.seed)
    } else if world.terrain_version >= 2 {
        biome_at_with_height_v2(x, z, world.seed, terrain_height_v2(x, z, world.seed))
    } else {
        legacy_biome_at(x, z, world.seed)
    }
}

pub(crate) fn terrain_height_for_version(x: i32, z: i32, seed: u64, version: u32) -> i32 {
    if version >= TERRAIN_VERSION {
        terrain_height(x, z, seed)
    } else if version >= 2 {
        terrain_height_v2(x, z, seed)
    } else {
        legacy_terrain_height(x, z, seed)
    }
}

pub(crate) fn world_spawn_point(seed: u64, version: u32) -> (f32, i32, f32) {
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

pub(crate) fn dry_spawn_column(seed: u64, version: u32, x: i32, z: i32) -> Option<i32> {
    let (height, has_water) = if version >= TERRAIN_VERSION {
        let (height, water_level) = world::terrain::terrain_profile(x, z, seed);
        (height, water_level.is_some() || height < SEA_LEVEL)
    } else {
        let height = terrain_height_for_version(x, z, seed, version);
        (height, height < SEA_LEVEL)
    };
    (!has_water).then_some(height)
}

pub(crate) fn spawn_site_is_dry(seed: u64, version: u32, x: i32, z: i32) -> bool {
    (-1..=1).all(|dz| {
        (-1..=1).all(|dx| {
            dry_spawn_column(seed, version, x + dx, z + dz)
                .is_some_and(|height| height > SEA_LEVEL + 1)
        })
    })
}

pub(crate) fn spawn_site_has_relief(seed: u64, x: i32, z: i32) -> bool {
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

pub(crate) fn spawn_site_has_block_variety(seed: u64, x: i32, z: i32) -> bool {
    let biome = biome_at(x, z, seed);
    let height = terrain_height(x, z, seed);
    if height <= SEA_LEVEL + 5 {
        return false;
    }
    // Reject solid-rock caps and require a real surface cap above deeper stone.
    !matches!(biome, Biome::Volcano | Biome::Cave)
        && (biome.surface_block() != STONE || biome == Biome::Mountain)
}
