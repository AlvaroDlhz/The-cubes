//! Creatures: spawning, behaviour, rewards and ray targeting.

use crate::*;

pub(crate) const MAX_MOBS: usize = 8;

pub(crate) const MAX_PASSIVE_MOBS: usize = 5;

pub(crate) const MAX_HOSTILE_MOBS: usize = 3;

pub(crate) const MOB_SPAWN_INTERVAL: f32 = 4.0;

pub(crate) const MOB_DESPAWN_DISTANCE: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MobKind {
    Grazer,
    NightStalker,
}

#[derive(Component)]
pub(crate) struct Mob {
    pub(crate) kind: MobKind,
    pub(crate) health: u8,
    pub(crate) attack_elapsed: f32,
    pub(crate) wander_elapsed: f32,
    pub(crate) wander_angle: f32,
    pub(crate) wander_step: u32,
}

#[derive(Resource, Default)]
pub(crate) struct MobPopulation {
    pub(crate) spawn_elapsed: f32,
    pub(crate) spawn_attempt: u64,
}

#[derive(Resource, Default)]
pub(crate) struct MobAssets {
    pub(crate) cube: Option<Handle<Mesh>>,
    pub(crate) grazer_body: Option<Handle<StandardMaterial>>,
    pub(crate) grazer_head: Option<Handle<StandardMaterial>>,
    pub(crate) grazer_leg: Option<Handle<StandardMaterial>>,
    pub(crate) stalker_body: Option<Handle<StandardMaterial>>,
    pub(crate) stalker_eye: Option<Handle<StandardMaterial>>,
}

pub(crate) fn update_mobs(
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

pub(crate) fn is_night(daylight: f32) -> bool {
    daylight < 0.08
}

pub(crate) fn can_spawn_mob(kind: MobKind, total: usize, grazers: usize, stalkers: usize) -> bool {
    total < MAX_MOBS
        && match kind {
            MobKind::Grazer => grazers < MAX_PASSIVE_MOBS,
            MobKind::NightStalker => stalkers < MAX_HOSTILE_MOBS,
        }
}

pub(crate) fn find_safe_spawn_position(world: &WorldData, mut position: Vec3) -> Vec3 {
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

pub(crate) fn mob_ground_height(world: &WorldData, x: i32, z: i32) -> Option<i32> {
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

pub(crate) fn mob_spawn_position(
    world: &WorldData,
    player_position: Vec3,
    attempt: u64,
) -> Option<Vec3> {
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

pub(crate) fn mob_ground_position(world: &WorldData, x: i32, z: i32) -> Option<Vec3> {
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

pub(crate) fn is_mob_clear_block(block: u8) -> bool {
    matches!(block, AIR | RED_FLOWER | YELLOW_FLOWER | TALL_GRASS)
}

pub(crate) fn spawn_mob(
    commands: &mut Commands,
    assets: &MobAssets,
    kind: MobKind,
    position: Vec3,
) {
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

pub(crate) fn mob_rewards(kind: MobKind) -> (u32, u32) {
    match kind {
        MobKind::Grazer => (2, 3),
        MobKind::NightStalker => (0, 5),
    }
}

pub(crate) fn ray_sphere_distance(
    origin: Vec3,
    direction: Vec3,
    center: Vec3,
    radius: f32,
) -> Option<f32> {
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
