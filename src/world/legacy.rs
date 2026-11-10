//! Terrain generators kept so worlds created by older versions still load.

use crate::*;

pub(crate) fn generate_legacy_chunk(coord: IVec3, seed: u64) -> Chunk {
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

pub(crate) fn generate_v2_chunk(coord: IVec3, seed: u64) -> Chunk {
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

pub(crate) fn legacy_biome_at(x: i32, z: i32, seed: u64) -> Biome {
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

pub(crate) fn legacy_terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    let broad = value_noise(x, z, seed ^ 0x9E37_79B9_7F4A_7C15, 0.035);
    let detail = value_noise(x, z, seed ^ 0xBF58_476D_1CE4_E5B9, 0.11);
    let mountain = value_noise(x, z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.012);
    let mountain_weight = smoothstep(0.64, 0.80, mountain);
    (4.0 + (broad - 0.5) * 8.0 + (detail - 0.5) * 2.0 + mountain_weight * 9.0)
        .round()
        .clamp(1.0, (CHUNK_SIZE - 1) as f32) as i32
}

pub(crate) fn biome_at_with_height_v2(x: i32, z: i32, seed: u64, height: i32) -> Biome {
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

pub(crate) fn terrain_height_v2(x: i32, z: i32, seed: u64) -> i32 {
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

pub(crate) fn is_cave_v2(x: i32, y: i32, z: i32, seed: u64) -> bool {
    if y <= 5 {
        return false;
    }
    let tunnel = value_noise_3d(x, y, z, seed ^ 0xC0FF_EE12_3456_7890, 0.035);
    let chamber = value_noise_3d(x, y, z, seed ^ 0x91E1_0DA5_C79E_7B1D, 0.065);
    (tunnel - 0.5).abs() < 0.045 && chamber > 0.38
}

pub(crate) fn value_noise(x: i32, z: i32, seed: u64, scale: f32) -> f32 {
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

pub(crate) fn value_noise_3d(x: i32, y: i32, z: i32, seed: u64, scale: f32) -> f32 {
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

pub(crate) fn hash_value(x: i32, z: i32, seed: u64) -> f32 {
    (coordinate_hash(x, z, seed) >> 40) as f32 / (1_u32 << 24) as f32
}

pub(crate) fn hash_value_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (z as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value >> 40) as f32 / (1_u32 << 24) as f32
}

pub(crate) fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}
