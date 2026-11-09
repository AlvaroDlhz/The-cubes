use crate::blocks::*;
use crate::world::Chunk;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Plains,
    Forest,
    Desert,
    Mountain,
    Tundra,
    Beach,
    Swamp,
    Volcano,
    Cave,
}

impl Biome {
    pub fn surface_block(self) -> u8 {
        match self {
            Self::Plains | Self::Forest => GRASS,
            Self::Desert => SAND,
            Self::Mountain | Self::Volcano => STONE,
            Self::Tundra => SNOW,
            Self::Beach => SAND,
            Self::Swamp => MUD,
            Self::Cave => STONE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Plains => "Pradera",
            Self::Forest => "Bosque",
            Self::Desert => "Desierto",
            Self::Mountain => "Montaña",
            Self::Tundra => "Tundra",
            Self::Beach => "Playa",
            Self::Swamp => "Pantano",
            Self::Volcano => "Volcán",
            Self::Cave => "Cueva",
        }
    }
}

pub fn generate_chunk(coord: IVec3, seed: u64) -> Chunk {
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
                } else if biome == Biome::Tundra && y >= height - 2 {
                    if y == height {
                        SNOW
                    } else {
                        ICE
                    }
                } else if biome == Biome::Beach && y >= height - 2 {
                    SAND
                } else if biome == Biome::Swamp && y >= height - 2 {
                    if y == height {
                        MUD
                    } else {
                        DIRT
                    }
                } else if biome == Biome::Volcano && y >= height - 2 {
                    if coordinate_hash(world_x, world_z, seed ^ 0x9999) % 100 < 15 {
                        OBSIDIAN
                    } else {
                        STONE
                    }
                } else if biome == Biome::Cave {
                    if y < height / 3 {
                        if coordinate_hash_3d(world_x, y, world_z, seed ^ 0x3333) % 100 < 8 {
                            OBSIDIAN
                        } else {
                            STONE
                        }
                    } else {
                        STONE
                    }
                } else if y >= height - 3 {
                    DIRT
                } else {
                    STONE
                };
                chunk.set(x, y, z, block);
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
                            if chunk.get(local.x, local.y, local.z) == AIR {
                                chunk.set(local.x, local.y, local.z, LEAVES);
                                // Occasionally spawn apples in leaves
                                if coordinate_hash(world_x ^ leaf_x, leaf_y, seed ^ 0x5C0F) % 100 < 15 {
                                    chunk.set(local.x, local.y, local.z, APPLE);
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
                    chunk.set(x, mushroom_y, z, MUSHROOM);
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
                    chunk.set(x, wheat_y, z, WHEAT);
                }
            }
        }
    }

    // Generate tundra ice ponds
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Tundra {
                continue;
            }
            // Sparse ice patches on surface
            if coordinate_hash(world_x, world_z, seed ^ 0x4444_5555) % 100 < 3 {
                let height = terrain_height(world_x, world_z, seed);
                let ice_y = height + 1;
                if (0..CHUNK_SIZE).contains(&ice_y) {
                    chunk.set(x, ice_y, z, ICE);
                }
            }
        }
    }

    // Generate swamp vegetation
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Swamp {
                continue;
            }
            let height = terrain_height(world_x, world_z, seed);
            // Mushrooms are common in swamps
            if coordinate_hash(world_x, world_z, seed ^ 0x6666_7777) % 100 < 12 {
                let mushroom_y = height + 1;
                if (0..CHUNK_SIZE).contains(&mushroom_y) {
                    chunk.set(x, mushroom_y, z, MUSHROOM);
                }
            }
        }
    }

    // Generate volcano lava vents
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Volcano {
                continue;
            }
            let height = terrain_height(world_x, world_z, seed);
            // Lava pockets near the top
            if coordinate_hash(world_x, world_z, seed ^ 0x8888_9999) % 100 < 6 {
                let lava_y = height.saturating_sub(2);
                if (0..CHUNK_SIZE).contains(&lava_y) && lava_y > 5 {
                    chunk.set(x, lava_y, z, LAVA);
                }
            }
        }
    }

    // Generate cave minerals (sparse gravel and obsidian)
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            if biome_at(world_x, world_z, seed) != Biome::Cave {
                continue;
            }
            let height = terrain_height(world_x, world_z, seed);
            // Gravel in cave floors
            if coordinate_hash(world_x, world_z, seed ^ 0xAAAA_BBBB) % 100 < 8 {
                let gravel_y = height.saturating_sub(3);
                if (0..CHUNK_SIZE).contains(&gravel_y) {
                    chunk.set(x, gravel_y, z, GRAVEL);
                }
            }
        }
    }
    chunk
}

pub fn biome_at(x: i32, z: i32, seed: u64) -> Biome {
    let temperature = value_noise(x, z, seed ^ 0xA076_1D64_78BD_642F, 0.012);
    let moisture = value_noise(x, z, seed ^ 0xE703_7ED1_A0B4_28DB, 0.018);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.012);
    let elevation = value_noise(x, z, seed ^ 0xDEAD_BEEF_CAFE_BABE, 0.008);
    let cave_factor = value_noise(x, z, seed ^ 0x1111_2222_3333_4444, 0.025);
    
    let mountain_weight = smoothstep(0.64, 0.80, mountain);
    let desert_weight =
        smoothstep(0.58, 0.78, temperature) * (1.0 - smoothstep(0.38, 0.60, moisture));
    let forest_weight =
        smoothstep(0.54, 0.72, moisture) * (1.0 - smoothstep(0.72, 0.88, temperature));
    let tundra_weight = smoothstep(0.78, 0.92, temperature).max(0.0);
    let swamp_weight = smoothstep(0.68, 0.78, moisture) * smoothstep(0.38, 0.52, temperature);
    let beach_weight = smoothstep(0.48, 0.56, elevation);
    let volcano_weight = smoothstep(0.80, 0.92, mountain) * smoothstep(0.12, 0.20, temperature);

    // Caves have priority if cave factor is high
    if cave_factor > 0.75 && elevation < 0.45 {
        return Biome::Cave;
    }

    if volcano_weight > 0.35 {
        Biome::Volcano
    } else if tundra_weight > 0.65 {
        Biome::Tundra
    } else if mountain_weight > 0.55 {
        Biome::Mountain
    } else if beach_weight > 0.40 {
        Biome::Beach
    } else if desert_weight > 0.15 && desert_weight >= forest_weight {
        Biome::Desert
    } else if swamp_weight > 0.15 {
        Biome::Swamp
    } else if forest_weight > 0.28 {
        Biome::Forest
    } else {
        Biome::Plains
    }
}

pub fn terrain_height(x: i32, z: i32, seed: u64) -> i32 {
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

pub fn coordinate_hash(x: i32, z: i32, seed: u64) -> u64 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub fn coordinate_hash_3d(x: i32, y: i32, z: i32, seed: u64) -> u64 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (z as i64 as u64).wrapping_mul(0x85EB_CA6B_7EA7_735F);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn set_generated_block(chunk: &mut Chunk, x: i32, y: i32, z: i32, block: u8) {
    if (0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y) && (0..CHUNK_SIZE).contains(&z)
    {
        chunk.set(x, y, z, block);
    }
}
