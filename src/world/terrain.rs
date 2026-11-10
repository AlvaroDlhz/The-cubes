use crate::blocks::*;
use crate::world::Chunk;
use bevy::prelude::*;

#[path = "biomes/mod.rs"]
pub mod biomes;

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
            Self::Desert | Self::Beach => SAND,
            Self::Mountain | Self::Volcano | Self::Cave => STONE,
            Self::Tundra => SNOW,
            Self::Swamp => MUD,
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

#[derive(Clone, Copy)]
pub struct TerrainBlockPalette {
    pub air: u8,
    pub grass: u8,
    pub dirt: u8,
    pub stone: u8,
    pub sand: u8,
    pub wood: u8,
    pub leaves: u8,
    pub apple: u8,
    pub wheat: u8,
    pub mushroom: u8,
    pub water: u8,
    pub red_flower: u8,
    pub yellow_flower: u8,
    pub tall_grass: u8,
    pub planks: u8,
}

impl Default for TerrainBlockPalette {
    fn default() -> Self {
        Self {
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
            red_flower: crate::blocks::RED_FLOWER,
            yellow_flower: crate::blocks::YELLOW_FLOWER,
            tall_grass: crate::blocks::TALL_GRASS,
            planks: PLANKS,
        }
    }
}

pub fn generate_chunk(coord: IVec3, seed: u64) -> Chunk {
    generate_chunk_with_palette(coord, seed, TerrainBlockPalette::default())
}

pub fn generate_chunk_with_palette(coord: IVec3, seed: u64, blocks: TerrainBlockPalette) -> Chunk {
    const COLUMN_SIDE: usize = (CHUNK_SIZE + 4) as usize;
    const COLUMN_COUNT: usize = COLUMN_SIDE * COLUMN_SIDE;
    const WORLD_HEIGHT: i32 = 128;
    const SEA_LEVEL: i32 = 48;
    let mut chunk = Chunk::new();
    let base_y = coord.y * CHUNK_SIZE;
    let mut columns = [(0, Biome::Plains, None); COLUMN_COUNT];

    for z in -2..CHUNK_SIZE + 2 {
        for x in -2..CHUNK_SIZE + 2 {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let (height, mut water_level) = terrain_profile(world_x, world_z, seed);
            let biome = biome_at_with_height(world_x, world_z, seed, height);
            if height < SEA_LEVEL {
                water_level = Some(water_level.unwrap_or(SEA_LEVEL).max(SEA_LEVEL));
            }
            columns[((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize] =
                (height, biome, water_level);
        }
    }

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let (height, biome, water_level) =
                columns[((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize];
            for local_y in 0..CHUNK_SIZE {
                let world_y = base_y + local_y;
                if !(0..WORLD_HEIGHT).contains(&world_y) {
                    continue;
                }
                if world_y <= height {
                    if world_y < height - 4 && is_cave(world_x, world_y, world_z, seed) {
                        if world_y <= SEA_LEVEL && world_y > 5 {
                            chunk.set(x, local_y, z, blocks.water);
                        }
                        continue;
                    }
                    let depth = height - world_y;
                    let block = biomes::surface_block(biome, depth, blocks);
                    chunk.set(x, local_y, z, block);
                } else if water_level.is_some_and(|level| world_y <= level)
                    || world_y <= SEA_LEVEL && height < SEA_LEVEL
                {
                    chunk.set(x, local_y, z, blocks.water);
                }
            }
        }
    }

    // Grove noise creates forest patches separated by clearings.
    for z in -2..CHUNK_SIZE + 2 {
        for x in -2..CHUNK_SIZE + 2 {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let (height, biome, _) = columns[((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize];
            let grove = feature_noise(world_x, world_z, seed ^ 0xA24B_AED4_963E_E407, 0.024);
            let tree_roll = coordinate_hash(world_x, world_z, seed ^ 0x71A5) % 100;
            if height <= SEA_LEVEL || !biomes::spawns_tree(biome, grove, tree_roll) {
                continue;
            }
            let tree_hash = coordinate_hash(world_x, world_z, seed ^ 0x19D3);
            let tree_height = biomes::tree_height(biome, tree_hash);
            for y in height + 1..=height + tree_height {
                set_generated_block(&mut chunk, x, y - base_y, z, blocks.wood);
            }
            let crown_y = height + tree_height;
            for y in crown_y - 2..=crown_y + 1 {
                let radius: i32 = if y == crown_y + 1 { 1 } else { 2 };
                for dz in -radius..=radius {
                    for dx in -radius..=radius {
                        if y == crown_y + 1 && dx.abs() + dz.abs() > 1 {
                            continue;
                        }
                        let lx = x + dx;
                        let lz = z + dz;
                        let ly = y - base_y;
                        if !(0..CHUNK_SIZE).contains(&lx)
                            || !(0..CHUNK_SIZE).contains(&lz)
                            || !(0..CHUNK_SIZE).contains(&ly)
                            || chunk.get(lx, ly, lz) != blocks.air
                        {
                            continue;
                        }
                        let leaf =
                            if coordinate_hash(world_x + dx, world_z + dz, seed ^ 0x5C0F) % 100 < 3
                            {
                                blocks.apple
                            } else {
                                blocks.leaves
                            };
                        chunk.set(lx, ly, lz, leaf);
                    }
                }
            }
        }
    }

    // Flowers and mushrooms follow broad patches instead of uniform scatter.
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = coord.x * CHUNK_SIZE + x;
            let world_z = coord.z * CHUNK_SIZE + z;
            let (height, biome, _) = columns[((z + 2) as usize) * COLUMN_SIDE + (x + 2) as usize];
            if height <= SEA_LEVEL {
                continue;
            }
            let patch = feature_noise(world_x, world_z, seed ^ 0xDEAD_BEEF, 0.045);
            let roll = coordinate_hash(world_x, world_z, seed ^ 0xBEEF_FACE) % 100;
            let plant = biomes::plant_block(biome, patch, roll, blocks);
            if plant != blocks.air && chunk.get(x, height + 1 - base_y, z) == blocks.air {
                set_generated_block(&mut chunk, x, height + 1 - base_y, z, plant);
            }
        }
    }

    // Mountain boulders and rare stone ruins form deterministic landmarks.
    let min_cell_x = (coord.x * CHUNK_SIZE - 3).div_euclid(12);
    let max_cell_x = (coord.x * CHUNK_SIZE + CHUNK_SIZE + 2).div_euclid(12);
    let min_cell_z = (coord.z * CHUNK_SIZE - 3).div_euclid(12);
    let max_cell_z = (coord.z * CHUNK_SIZE + CHUNK_SIZE + 2).div_euclid(12);
    for cz in min_cell_z..=max_cell_z {
        for cx in min_cell_x..=max_cell_x {
            let hash = coordinate_hash(cx, cz, seed ^ 0x8F3F_73B5_CF1C_9ADE);
            let wx = cx * 12 + ((hash >> 8) % 12) as i32;
            let wz = cz * 12 + ((hash >> 16) % 12) as i32;
            let h = terrain_height(wx, wz, seed);
            let biome = biome_at_with_height(wx, wz, seed, h);
            if !biomes::spawns_boulder(biome, hash) {
                continue;
            }
            let radius = 1 + ((hash >> 24) % 2) as i32;
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() + dz.abs() > radius + 1 {
                        continue;
                    }
                    let lx = wx + dx - coord.x * CHUNK_SIZE;
                    let lz = wz + dz - coord.z * CHUNK_SIZE;
                    set_generated_block(&mut chunk, lx, h + 1 - base_y, lz, blocks.stone);
                    if (dx.abs() + dz.abs()) % 2 == 0 {
                        set_generated_block(&mut chunk, lx, h + 2 - base_y, lz, blocks.stone);
                    }
                }
            }
        }
    }

    let structure_cell = 96;
    for cz in (coord.z * CHUNK_SIZE).div_euclid(structure_cell) - 1
        ..=(coord.z * CHUNK_SIZE + CHUNK_SIZE).div_euclid(structure_cell) + 1
    {
        for cx in (coord.x * CHUNK_SIZE).div_euclid(structure_cell) - 1
            ..=(coord.x * CHUNK_SIZE + CHUNK_SIZE).div_euclid(structure_cell) + 1
        {
            let hash = coordinate_hash(cx, cz, seed ^ 0x510E_527F_ADE6_82D1);
            let wx = cx * structure_cell + ((hash >> 8) % structure_cell as u64) as i32;
            let wz = cz * structure_cell + ((hash >> 24) % structure_cell as u64) as i32;
            let h = terrain_height(wx, wz, seed);
            let biome = biome_at_with_height(wx, wz, seed, h);
            let is_cabin = biomes::spawns_cabin(biome);
            let chance = if is_cabin { 7 } else { 18 };
            if h <= SEA_LEVEL || hash % 100 >= chance || (!is_cabin && !biomes::spawns_ruin(biome))
            {
                continue;
            }
            let flat_enough = (-2..=2).all(|dz| {
                (-2..=2).all(|dx| (terrain_height(wx + dx, wz + dz, seed) - h).abs() <= 2)
            });
            if !flat_enough {
                continue;
            }
            let lx = wx - coord.x * CHUNK_SIZE;
            let lz = wz - coord.z * CHUNK_SIZE;
            if is_cabin {
                generate_plains_cabin(&mut chunk, lx, h - base_y, lz, blocks);
            } else {
                for dx in 0..5 {
                    for dz in 0..5 {
                        if dx == 0 || dx == 4 || dz == 0 || dz == 4 {
                            let wall = if (dx == 0 || dx == 4) && (dz == 0 || dz == 4) {
                                3
                            } else {
                                1
                            };
                            for dy in 1..=wall {
                                set_generated_block(
                                    &mut chunk,
                                    lx + dx,
                                    h + dy - base_y,
                                    lz + dz,
                                    blocks.stone,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    generate_ore_veins(&mut chunk, coord, seed);
    chunk
}

fn generate_ore_veins(chunk: &mut Chunk, coord: IVec3, seed: u64) {
    const CELL_XZ: i32 = 8;
    const CELL_Y: i32 = 6;
    let base_x = coord.x * CHUNK_SIZE;
    let base_y = coord.y * CHUNK_SIZE;
    let base_z = coord.z * CHUNK_SIZE;

    for (block, min_y, max_y, chance, salt) in [
        (COAL_ORE, 5_i32, 112_i32, 48_u64, 0xC01A_0100),
        (COPPER_ORE, 20, 80, 34_u64, 0xC022_E200),
        (IRON_ORE, 5, 64, 30_u64, 0x1A0F_0100),
    ] {
        let min_cell_y = min_y.div_euclid(CELL_Y);
        let max_cell_y = max_y.div_euclid(CELL_Y);
        let min_cell_x = base_x.div_euclid(CELL_XZ) - 1;
        let max_cell_x = (base_x + CHUNK_SIZE - 1).div_euclid(CELL_XZ) + 1;
        let min_cell_z = base_z.div_euclid(CELL_XZ) - 1;
        let max_cell_z = (base_z + CHUNK_SIZE - 1).div_euclid(CELL_XZ) + 1;

        for cell_y in min_cell_y..=max_cell_y {
            for cell_z in min_cell_z..=max_cell_z {
                for cell_x in min_cell_x..=max_cell_x {
                    let hash = coordinate_hash_3d(cell_x, cell_y, cell_z, seed ^ salt);
                    if hash % 100 >= chance {
                        continue;
                    }
                    let center = IVec3::new(
                        cell_x * CELL_XZ + ((hash >> 8) % CELL_XZ as u64) as i32,
                        cell_y * CELL_Y + ((hash >> 16) % CELL_Y as u64) as i32,
                        cell_z * CELL_XZ + ((hash >> 24) % CELL_XZ as u64) as i32,
                    );
                    if !(min_y..=max_y).contains(&center.y) {
                        continue;
                    }
                    let radius = if block == COAL_ORE {
                        2
                    } else if (hash >> 32) % 3 == 0 {
                        2
                    } else {
                        1
                    };

                    for dy in -radius..=radius {
                        for dz in -radius..=radius {
                            for dx in -radius..=radius {
                                if dx * dx + dy * dy + dz * dz > radius * radius + 1 {
                                    continue;
                                }
                                let world_x = center.x + dx;
                                let world_y = center.y + dy;
                                let world_z = center.z + dz;
                                if !(min_y..=max_y).contains(&world_y)
                                    || !(base_x..base_x + CHUNK_SIZE).contains(&world_x)
                                    || !(base_y..base_y + CHUNK_SIZE).contains(&world_y)
                                    || !(base_z..base_z + CHUNK_SIZE).contains(&world_z)
                                    || world_y > terrain_height(world_x, world_z, seed) - 4
                                {
                                    continue;
                                }
                                let local_x = world_x - base_x;
                                let local_y = world_y - base_y;
                                let local_z = world_z - base_z;
                                if chunk.get(local_x, local_y, local_z) == STONE {
                                    chunk.set(local_x, local_y, local_z, block);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn generate_plains_cabin(
    chunk: &mut Chunk,
    x: i32,
    ground_y: i32,
    z: i32,
    blocks: TerrainBlockPalette,
) {
    // A tiny deterministic cabin, with a doorway and a flat roof, built only
    // on broad, gently sloped grassland clearings.
    for dx in 0..5 {
        for dz in 0..5 {
            set_generated_block(chunk, x + dx, ground_y + 1, z + dz, blocks.planks);
            let border = dx == 0 || dx == 4 || dz == 0 || dz == 4;
            if !border {
                continue;
            }
            for dy in 1..=3 {
                let doorway = dz == 0 && dx == 2 && dy <= 2;
                let window =
                    dy == 2 && ((dx == 0 || dx == 4) && dz == 2 || (dz == 0 || dz == 4) && dx == 1);
                if doorway || window {
                    continue;
                }
                let block = if (dx == 0 || dx == 4) && (dz == 0 || dz == 4) {
                    blocks.wood
                } else {
                    blocks.planks
                };
                set_generated_block(chunk, x + dx, ground_y + 1 + dy, z + dz, block);
            }
            set_generated_block(chunk, x + dx, ground_y + 5, z + dz, blocks.planks);
        }
    }
    for dx in 1..4 {
        for dz in 1..4 {
            set_generated_block(chunk, x + dx, ground_y + 5, z + dz, blocks.planks);
        }
    }
}

pub fn biome_at(x: i32, z: i32, seed: u64) -> Biome {
    biome_at_with_height(x, z, seed, terrain_height(x, z, seed))
}

pub fn biome_at_with_height(x: i32, z: i32, seed: u64, height: i32) -> Biome {
    if (height - 48).abs() <= 2 {
        return Biome::Beach;
    }
    let (warped_x, warped_z) = warped_position(x, z, seed);
    let temperature = fractal_noise(warped_x, warped_z, seed ^ 0xA076_1D64_78BD_642F, 0.0022, 3);
    let moisture = fractal_noise(warped_x, warped_z, seed ^ 0xE703_7ED1_A0B4_28DB, 0.0032, 3);
    let ridges = ridge_noise(warped_x, warped_z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.004);
    biomes::classify(height, temperature, moisture, ridges)
}

pub fn terrain_height(x: i32, z: i32, seed: u64) -> i32 {
    terrain_profile(x, z, seed).0
}

pub fn terrain_profile(x: i32, z: i32, seed: u64) -> (i32, Option<i32>) {
    let (warped_x, warped_z) = warped_position(x, z, seed);
    let continent = fractal_noise(warped_x, warped_z, seed ^ 0x9E37_79B9_7F4A_7C15, 0.0018, 4);
    let rolling = fractal_noise(warped_x, warped_z, seed ^ 0xBF58_476D_1CE4_E5B9, 0.007, 4);
    let detail = fractal_noise(x, z, seed ^ 0x94D0_49BB_1331_11EB, 0.035, 3);
    let mountain_signal =
        fractal_noise(warped_x, warped_z, seed ^ 0x8EBC_6AF0_9C88_C6E3, 0.0035, 4);
    let mountain_region = fractal_noise(x, z, seed ^ 0xDB4F_0B91_75AE_2165, 0.0017, 3);
    let ridges = 1.0 - (mountain_signal * 2.0 - 1.0).abs();
    let mountain_weight = smoothstep(0.54, 0.76, mountain_region);
    let land_weight = smoothstep(0.28, 0.68, continent);
    let mut base_height = (34.0
        + land_weight * 30.0
        + (rolling - 0.5) * 20.0
        + (detail - 0.5) * 6.0
        + ridges * mountain_weight * 52.0)
        .round()
        .clamp(8.0, 112.0) as i32;

    let temperature = fractal_noise(warped_x, warped_z, seed ^ 0xA076_1D64_78BD_642F, 0.0022, 3);
    let moisture = fractal_noise(warped_x, warped_z, seed ^ 0xE703_7ED1_A0B4_28DB, 0.0032, 3);
    if biomes::classify(base_height, temperature, moisture, ridges) == Biome::Plains {
        // Grassland keeps the large landscape shape but softens local slopes
        // into broad, walkable rises and clearings.
        base_height = (34.0
            + land_weight * 30.0
            + (rolling - 0.5) * 14.0
            + (detail - 0.5) * 3.0
            + ridges * mountain_weight * 52.0)
            .round()
            .clamp(8.0, 112.0) as i32;
    }

    let river_noise = fractal_noise(warped_x, warped_z, seed ^ 0xD1B5_4A32_D192_ED03, 0.0032, 3);
    let river_distance = (river_noise - 0.5).abs();
    let river_mask = 1.0 - smoothstep(0.009, 0.032, river_distance);
    let mut height = base_height;
    let mut water_level = None;
    if river_mask > 0.08 && base_height > 50 {
        let level = base_height - 2;
        let depth = (river_mask * 7.0).round() as i32;
        height = (base_height - depth).min(level - 1);
        water_level = Some(level);
    }

    if let Some((lake_ground, lake_level)) = lake_profile(x, z, seed, base_height) {
        if lake_ground < height {
            height = lake_ground;
            if lake_level.is_some() {
                water_level = lake_level;
            }
        }
    }
    (height, water_level)
}

fn lake_profile(x: i32, z: i32, seed: u64, base_height: i32) -> Option<(i32, Option<i32>)> {
    const CELL: i32 = 128;
    let cell_x = x.div_euclid(CELL);
    let cell_z = z.div_euclid(CELL);
    let mut best: Option<(f32, f32, i32)> = None;
    for dz in -1..=1 {
        for dx in -1..=1 {
            let anchor_x = cell_x + dx;
            let anchor_z = cell_z + dz;
            let hash = coordinate_hash(anchor_x, anchor_z, seed ^ 0x6A09_E667_F3BC_C909);
            if hash % 4 != 0 {
                continue;
            }
            let center_x = anchor_x * CELL + ((hash >> 8) % CELL as u64) as i32;
            let center_z = anchor_z * CELL + ((hash >> 24) % CELL as u64) as i32;
            let radius = 9.0 + ((hash >> 40) % 12) as f32;
            let distance = Vec2::new((x - center_x) as f32, (z - center_z) as f32).length();
            if distance >= radius {
                continue;
            }
            let level = 49 + ((hash >> 52) % 8) as i32;
            if base_height <= level - 2 || base_height > level + 18 {
                continue;
            }
            if best.is_none_or(|(closest, _, _)| distance < closest) {
                best = Some((distance, radius, level));
            }
        }
    }
    let (distance, radius, level) = best?;
    let basin = 1.0 - smoothstep(radius - 4.0, radius, distance);
    let ground =
        (base_height as f32 + (level as f32 - 3.0 - base_height as f32) * basin).round() as i32;
    Some((ground, (distance < radius - 4.0).then_some(level)))
}

pub fn is_cave(x: i32, y: i32, z: i32, seed: u64) -> bool {
    if y <= 5 {
        return false;
    }
    let tunnel_a = value_noise_3d_anisotropic(x, y, z, seed ^ 0xC0FF_EE12_3456_7890, 0.038, 0.55);
    let tunnel_b =
        value_noise_3d_anisotropic(x + 97, y, z - 53, seed ^ 0x74A1_2C39_B5D6_EF08, 0.055, 0.45);
    let chamber = value_noise_3d(x, y, z, seed ^ 0x91E1_0DA5_C79E_7B1D, 0.022);
    let cavern_shape = value_noise_3d(x, y, z, seed ^ 0xA54F_F53A_5F1D_36F1, 0.043);
    (tunnel_a - 0.5).abs() < 0.035 && (tunnel_b - 0.5).abs() < 0.07
        || chamber > 0.70 && cavern_shape > 0.54
}

fn warped_position(x: i32, z: i32, seed: u64) -> (i32, i32) {
    let warp_x = value_noise(x, z, seed ^ 0x243F_6A88_85A3_08D3, 0.0018);
    let warp_z = value_noise(x, z, seed ^ 0x1319_8A2E_0370_7344, 0.0018);
    (
        x + ((warp_x - 0.5) * 72.0).round() as i32,
        z + ((warp_z - 0.5) * 72.0).round() as i32,
    )
}

fn fractal_noise(x: i32, z: i32, seed: u64, scale: f32, octaves: u32) -> f32 {
    let mut amplitude = 1.0;
    let mut frequency = scale;
    let mut total = 0.0;
    let mut weight = 0.0;
    for octave in 0..octaves {
        total += value_noise(
            x,
            z,
            seed ^ u64::from(octave).wrapping_mul(0x9E37_79B9),
            frequency,
        ) * amplitude;
        weight += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / weight
}

pub fn feature_noise(x: i32, z: i32, seed: u64, scale: f32) -> f32 {
    fractal_noise(x, z, seed, scale, 3)
}

fn ridge_noise(x: i32, z: i32, seed: u64, scale: f32) -> f32 {
    let signal = fractal_noise(x, z, seed, scale, 4);
    1.0 - (signal * 2.0 - 1.0).abs()
}

fn value_noise_3d(x: i32, y: i32, z: i32, seed: u64, scale: f32) -> f32 {
    value_noise_3d_anisotropic(x, y, z, seed, scale, 1.0)
}

fn value_noise_3d_anisotropic(
    x: i32,
    y: i32,
    z: i32,
    seed: u64,
    scale: f32,
    vertical_scale: f32,
) -> f32 {
    let scaled_x = x as f32 * scale;
    let scaled_y = y as f32 * scale * vertical_scale;
    let scaled_z = z as f32 * scale;
    let grid_x = scaled_x.floor() as i32;
    let grid_y = scaled_y.floor() as i32;
    let grid_z = scaled_z.floor() as i32;
    let fx = smoothstep(0.0, 1.0, scaled_x - grid_x as f32);
    let fy = smoothstep(0.0, 1.0, scaled_y - grid_y as f32);
    let fz = smoothstep(0.0, 1.0, scaled_z - grid_z as f32);
    let plane = |dy| {
        let x0z0 = hash_value_3d(grid_x, grid_y + dy, grid_z, seed);
        let x1z0 = hash_value_3d(grid_x + 1, grid_y + dy, grid_z, seed);
        let x0z1 = hash_value_3d(grid_x, grid_y + dy, grid_z + 1, seed);
        let x1z1 = hash_value_3d(grid_x + 1, grid_y + dy, grid_z + 1, seed);
        lerp(lerp(x0z0, x1z0, fx), lerp(x0z1, x1z1, fx), fz)
    };
    lerp(plane(0), plane(1), fy)
}

fn hash_value_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    (coordinate_hash_3d(x, y, z, seed) >> 40) as f32 / (1_u32 << 24) as f32
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
