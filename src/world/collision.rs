use crate::blocks::*;
use crate::world::WorldData;
use bevy::prelude::*;

pub fn block_at(world: &WorldData, pos: IVec3) -> u8 {
    let chunk_coord = IVec3::new(pos.x >> 4, 0, pos.z >> 4);
    let local_x = pos.x & 15;
    let local_z = pos.z & 15;
    let chunk = world.chunks.get(&chunk_coord).unwrap_or_else(|| {
        panic!("chunk not loaded at {:?}", chunk_coord);
    });
    chunk.get(local_x, pos.y, local_z)
}

pub fn set_block(world: &mut WorldData, pos: IVec3, block: u8) {
    let chunk_coord = IVec3::new(pos.x >> 4, 0, pos.z >> 4);
    let local_x = pos.x & 15;
    let local_z = pos.z & 15;
    if let Some(chunk) = world.chunks.get_mut(&chunk_coord) {
        chunk.set(local_x, pos.y, local_z, block);
        world.modified.insert(chunk_coord);
    }
}

pub fn collides_with_world(world: &WorldData, min: Vec3, max: Vec3) -> bool {
    let min_i = min.floor().as_ivec3();
    let max_i = max.ceil().as_ivec3();
    for x in min_i.x..=max_i.x {
        for y in min_i.y..=max_i.y {
            for z in min_i.z..=max_i.z {
                let block = block_at(world, IVec3::new(x, y, z));
                if is_block_solid(block, &world.door_states, IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn find_ceiling_or_ground(
    world: &WorldData,
    x: i32,
    z: i32,
    max_y: i32,
    step: i32,
) -> Option<i32> {
    (0..=max_y)
        .step_by(step as usize)
        .find(|&y| block_at(world, IVec3::new(x, y, z)) != AIR)
}

pub fn mob_ground_height(world: &WorldData, x: i32, z: i32) -> Option<i32> {
    find_ceiling_or_ground(world, x, z, 255, 1)
}

pub fn light_level_at(world: &WorldData, pos: IVec3) -> f32 {
    // Calculate sun light based on terrain height
    let terrain_h = crate::world::terrain::terrain_height(pos.x, pos.z, world.seed) as f32;
    let base_light = if pos.y as f32 > terrain_h {
        15.0
    } else {
        let depth_below_surface = terrain_h - pos.y as f32;
        (15.0 - depth_below_surface * 0.3).max(0.5)
    };

    // Add torch light contribution
    let mut torch_light: f32 = 0.0;
    let pos_f = pos.as_vec3();
    for torch_pos in world.door_states.keys() {
        // For simplicity, check if there are blocks around that could be torches
        // This is a placeholder - a full implementation would track torch positions separately
        let distance = pos_f.distance(torch_pos.as_vec3() + Vec3::new(0.5, 0.5, 0.5));
        if distance <= 8.0 {
            let contribution = (8.0 - distance) / 8.0;
            torch_light = torch_light.max(contribution);
        }
    }

    (base_light + torch_light).clamp(0.0, 15.0)
}
