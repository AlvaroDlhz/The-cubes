//! Breaking and placing blocks, mining visuals, drops and particles.

use crate::*;

pub(crate) const REACH: f32 = 6.0;

pub(crate) const MINING_CRACK_STAGES: usize = 8;

#[derive(Component)]
pub(crate) struct BlockParticle {
    pub(crate) lifetime: f32,
    pub(crate) velocity: Vec3,
}

#[derive(Component)]
pub(crate) struct BlockDrop {
    pub(crate) item: Item,
    pub(crate) velocity: Vec3,
    pub(crate) age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MiningTool {
    Pickaxe,
    Axe,
    Shovel,
}

#[derive(Clone, Copy)]
pub(crate) struct MiningTarget {
    pub(crate) position: IVec3,
    pub(crate) block: u8,
    pub(crate) tool: Option<MiningTool>,
    pub(crate) material: ToolMaterial,
    pub(crate) elapsed: f32,
    pub(crate) duration: f32,
}

#[derive(Resource, Default)]
pub(crate) struct MiningState {
    pub(crate) target: Option<MiningTarget>,
}

#[derive(Resource, Default)]
pub(crate) struct MiningVisuals {
    pub(crate) materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
pub(crate) struct MiningOverlay;

#[derive(Component)]
pub(crate) struct BlockOutline;

pub(crate) fn create_mining_materials(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let segments: [(i32, i32, i32, i32); 16] = [
        (16, 16, 14, 10),
        (14, 10, 11, 5),
        (16, 16, 21, 12),
        (21, 12, 27, 11),
        (16, 16, 20, 20),
        (20, 20, 22, 27),
        (16, 16, 11, 19),
        (11, 19, 5, 21),
        (14, 10, 17, 7),
        (11, 5, 8, 2),
        (21, 12, 23, 6),
        (27, 11, 30, 8),
        (20, 20, 26, 22),
        (22, 27, 19, 30),
        (11, 19, 10, 25),
        (5, 21, 2, 18),
    ];
    (0..MINING_CRACK_STAGES)
        .map(|stage| {
            let mut pixels = vec![0_u8; 32 * 32 * 4];
            for &(mut x, mut y, end_x, end_y) in &segments[..(stage + 1) * 2] {
                let dx: i32 = (end_x - x).abs();
                let dy: i32 = -(end_y - y).abs();
                let step_x = if x < end_x { 1 } else { -1 };
                let step_y = if y < end_y { 1 } else { -1 };
                let mut error = dx + dy;
                loop {
                    pixels[((y * 32 + x) * 4 + 3) as usize] = 220;
                    if x == end_x && y == end_y {
                        break;
                    }
                    let twice_error = 2 * error;
                    if twice_error >= dy {
                        error += dy;
                        x += step_x;
                    }
                    if twice_error <= dx {
                        error += dx;
                        y += step_y;
                    }
                }
            }
            let mut image = Image::new(
                Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect()
}

pub(crate) fn update_mining_visual(
    mining: Res<MiningState>,
    visuals: Res<MiningVisuals>,
    mut overlay: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<MiningOverlay>,
    >,
) {
    let Ok((mut transform, mut visibility, mut material)) = overlay.get_single_mut() else {
        return;
    };
    if let Some(target) = mining.target {
        let stage = ((target.elapsed / target.duration) * MINING_CRACK_STAGES as f32)
            .floor()
            .clamp(0.0, (MINING_CRACK_STAGES - 1) as f32) as usize;
        if let Some(handle) = visuals.materials.get(stage) {
            transform.translation = target.position.as_vec3() + Vec3::splat(0.5);
            material.0 = handle.clone();
            *visibility = Visibility::Visible;
        } else {
            error!("Mining overlay material for stage {stage} is missing");
            *visibility = Visibility::Hidden;
        }
    } else {
        *visibility = Visibility::Hidden;
    }
}

pub(crate) fn update_block_outline(
    world: Res<WorldData>,
    ui: Res<CraftingUi>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<&Transform, (With<Player>, Without<BlockOutline>)>,
    mut outline: Query<(&mut Transform, &mut Visibility), (With<BlockOutline>, Without<Player>)>,
) {
    let Ok((mut outline_transform, mut visibility)) = outline.get_single_mut() else {
        return;
    };
    if ui.is_active() {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok(window) = windows.get_single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok(camera) = camera.get_single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        *visibility = Visibility::Hidden;
        return;
    }
    let direction = camera.rotation * Vec3::NEG_Z;
    if let Some((target, _, _)) =
        raycast_with_distance(&world, camera.translation, direction, REACH)
    {
        outline_transform.translation = target.as_vec3() + Vec3::splat(0.5);
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}

pub(crate) fn reset_mining(
    mut mining: ResMut<MiningState>,
    // A single query: two `&mut Visibility` queries filtered only by `With`
    // are not provably disjoint and make Bevy panic (B0001) on first run.
    mut visuals: Query<&mut Visibility, Or<(With<MiningOverlay>, With<BlockOutline>)>>,
) {
    mining.target = None;
    for mut visibility in &mut visuals {
        *visibility = Visibility::Hidden;
    }
}

/// Create particle effects when a block is destroyed
pub(crate) fn block_drop_color(block_type: u8) -> Color {
    let color = block_definition(block_type).drop_color;
    Color::linear_rgb(color[0], color[1], color[2])
}

pub(crate) fn item_drop_color(item: Item) -> Color {
    match item {
        Item::Block(block) => block_drop_color(block),
        Item::Mineral(MineralKind::Coal) => Color::srgb(0.12, 0.12, 0.13),
        Item::Mineral(MineralKind::RawCopper) => Color::srgb(0.78, 0.42, 0.24),
        Item::Mineral(MineralKind::RawIron) => Color::srgb(0.70, 0.56, 0.45),
        Item::Mineral(MineralKind::CopperIngot) => Color::srgb(0.88, 0.50, 0.28),
        Item::Mineral(MineralKind::IronIngot) => Color::srgb(0.80, 0.80, 0.82),
        Item::Tool { .. } => Color::srgb(0.55, 0.58, 0.62),
        Item::BerryRation => Color::srgb(0.42, 0.62, 0.28),
        Item::Arrow => Color::srgb(0.72, 0.68, 0.55),
    }
}

pub(crate) fn inventory_item_for_block_drop(drop: BlockDropDefinition) -> Item {
    match drop {
        BlockDropDefinition::Block(block) => Item::Block(block),
        BlockDropDefinition::Mineral(mineral) => Item::Mineral(mineral),
    }
}

pub(crate) fn spawn_block_drop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    item: Item,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.28, 0.28, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: item_drop_color(item),
            ..default()
        })),
        Transform::from_translation(position),
        BlockDrop {
            item,
            velocity: Vec3::new(0.0, 2.0, 0.0),
            age: 0.0,
        },
    ));
}

pub(crate) fn update_block_drops(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<WorldData>,
    player: Query<&Transform, With<Player>>,
    mut inventory: ResMut<Inventory>,
    mut drops: Query<(Entity, &mut BlockDrop, &mut Transform), Without<Player>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let dt = time.delta_secs();
    // The player's Transform is at eye height; use the body center for pickup range.
    let pickup_target = player.translation - Vec3::Y * (PLAYER_EYE_HEIGHT - 0.5);
    for (entity, mut drop, mut transform) in &mut drops {
        drop.age += dt;
        let to_player = pickup_target - transform.translation;
        let distance = to_player.length();
        if drop.age > 0.45 && distance <= 1.9 {
            transform.translation += to_player.normalize_or_zero() * (6.0 * dt).min(distance);
            if transform.translation.distance(pickup_target) <= 0.35
                && inventory.try_add(drop.item, 1).is_ok()
            {
                commands.entity(entity).despawn();
                continue;
            }
        } else {
            let below = (transform.translation - Vec3::Y * 0.2).floor().as_ivec3();
            if block_at(&world, below) != AIR {
                drop.velocity.y = drop.velocity.y.max(0.0);
                transform.translation.y = transform.translation.y.max(below.y as f32 + 1.14);
            } else {
                drop.velocity.y -= 12.0 * dt;
                transform.translation.y += drop.velocity.y * dt;
            }
        }
        transform.rotate_y(dt * 1.8);
    }
}

pub(crate) fn spawn_block_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    block_pos: Vec3,
    block_type: u8,
) {
    // Spawn 5 particles per destroyed block
    let particle_count = 5;
    let block_color = block_drop_color(block_type);

    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let speed = 2.0 + (i as f32) * 0.3;
        let velocity = Vec3::new(
            angle.cos() * speed,
            3.0 + (i as f32) * 0.2,
            angle.sin() * speed,
        );

        let mesh = meshes.add(Cuboid::new(0.1, 0.1, 0.1));
        let material = materials.add(StandardMaterial {
            base_color: block_color,
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(block_pos + Vec3::splat(0.5)),
            BlockParticle {
                lifetime: 1.0,
                velocity,
            },
        ));
    }
}

pub(crate) fn update_particles(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut BlockParticle, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut transform) in &mut particles {
        particle.lifetime -= time.delta_secs();
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Physics: apply gravity and velocity
        let gravity = Vec3::new(0.0, -9.8, 0.0);
        let dt = time.delta_secs();
        particle.velocity += gravity * dt;
        transform.translation += particle.velocity * dt;
    }
}

pub(crate) fn edit_blocks(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    (time, mut mining): (Res<Time>, ResMut<MiningState>),
    camera: Query<&Transform, With<Player>>,
    mut mobs: Query<(Entity, &Transform, &mut Mob), Without<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
    mut crafting_ui: ResMut<CraftingUi>,
    chat: Res<ChatState>,
    (selected, selected_slot): (Res<SelectedBlock>, Res<SelectedHotbarSlot>),
    mut inventory: ResMut<Inventory>,
    mut survival: ResMut<Survival>,
    mut world: ResMut<WorldData>,
    mut storage: ResMut<WorldStorage>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if pointer.captured_this_frame || crafting_ui.is_active() || chat.open {
        mining.target = None;
        return;
    }
    let place = mouse.just_pressed(MouseButton::Right);
    let remove = mouse.pressed(MouseButton::Left) && !place;
    if !remove && !place {
        mining.target = None;
        return;
    }
    let Ok(window) = windows.get_single() else {
        mining.target = None;
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        mining.target = None;
        return;
    }
    let Ok(transform) = camera.get_single() else {
        mining.target = None;
        return;
    };

    if place {
        mining.target = None;
        if inventory.slots[selected_slot.0].is_some_and(|stack| stack.item == Item::BerryRation)
            && survival.hunger < MAX_HUNGER
        {
            item_inventory::take_one(&mut inventory.slots[selected_slot.0]);
            survival.hunger =
                (survival.hunger + FoodType::BerryRation.saturation()).min(MAX_HUNGER);
            survival.notice = "Comiste una ración.".into();
            survival.notice_elapsed = 2.0;
            return;
        }
    }
    let direction = transform.rotation * Vec3::NEG_Z;
    let block_hit = raycast_with_distance(&world, transform.translation, direction, REACH);
    if remove {
        let mob_hit = mobs
            .iter_mut()
            .filter_map(|(entity, mob_transform, mob)| {
                ray_sphere_distance(
                    transform.translation,
                    direction,
                    mob_transform.translation + Vec3::Y * 0.8,
                    0.7,
                )
                .map(|distance| (entity, mob.kind, distance))
            })
            .min_by(|left, right| left.2.total_cmp(&right.2));
        if let Some((entity, kind, distance)) = mob_hit {
            if block_hit.is_none_or(|(_, _, block_distance)| distance <= block_distance) {
                mining.target = None;
                if !mouse.just_pressed(MouseButton::Left) {
                    return;
                }
                let Ok((_, _, mut mob)) = mobs.get_mut(entity) else {
                    return;
                };
                let sword_material = match inventory.slots[selected_slot.0] {
                    Some(ItemStack {
                        item:
                            Item::Tool {
                                kind: ToolKind::Sword,
                                material,
                                ..
                            },
                        ..
                    }) => Some(material),
                    _ => None,
                };
                let sword = sword_material.is_some();
                let remaining = if sword {
                    wear_selected_tool(&mut inventory, selected_slot.0)
                } else {
                    None
                };
                let damage = sword_material.map_or(1, ToolMaterial::sword_damage);
                mob.health = mob.health.saturating_sub(damage);
                if mob.health == 0 {
                    commands.entity(entity).despawn_recursive();
                    let (food, experience) = mob_rewards(kind);
                    survival.food = survival.food.saturating_add(food).min(99);
                    survival.experience = survival.experience.saturating_add(experience);
                    if remaining == Some(0) {
                        survival.notice = "La espada se rompió.".into();
                        survival.notice_elapsed = 4.0;
                    } else {
                        survival.notice = match kind {
                            MobKind::Grazer if food > 0 => {
                                "Conseguiste carne y experiencia.".into()
                            }
                            MobKind::Grazer => "Derrotaste al animal; la comida está llena.".into(),
                            MobKind::NightStalker => {
                                "Derrotaste al acechador y ganaste experiencia.".into()
                            }
                        };
                        survival.notice_elapsed = 4.0;
                    }
                } else {
                    survival.notice = match kind {
                        MobKind::Grazer => "Has golpeado al animal.".into(),
                        MobKind::NightStalker => "Has golpeado al acechador.".into(),
                    };
                    survival.notice_elapsed = 1.5;
                }
                return;
            }
        }
    }
    let Some((hit, previous, _)) = block_hit else {
        mining.target = None;
        return;
    };

    if remove {
        let block = block_at(&world, hit);
        let tool = selected_mining_tool(block, inventory.slots[selected_slot.0]);
        let material = selected_tool_material(inventory.slots[selected_slot.0]);
        let definition = block_definition(block);
        if definition.requires_pickaxe {
            let notice = if tool != Some(MiningTool::Pickaxe) {
                Some("Necesitas un pico para extraer este mineral.")
            } else if material < definition.pickaxe_tier {
                Some("Necesitas un pico de mejor material para este mineral.")
            } else {
                None
            };
            if let Some(notice) = notice {
                mining.target = None;
                survival.notice = notice.into();
                survival.notice_elapsed = 2.0;
                return;
            }
        }
        let duration = mining_duration_with(block, tool, material);
        let fresh = MiningTarget {
            position: hit,
            block,
            tool,
            material,
            elapsed: 0.0,
            duration,
        };
        let target = mining.target.get_or_insert(fresh);
        if target.position != hit
            || target.block != block
            || target.tool != tool
            || target.material != material
        {
            *target = fresh;
        }
        target.elapsed += time.delta_secs();
        if target.elapsed < target.duration {
            return;
        }
        mining.target = None;
    }

    // Check for door interaction (right click on door)
    if place {
        let (door_chunk_coord, door_local) = block_address(hit);
        if let Some(door_chunk) = world.chunks.get(&door_chunk_coord) {
            let current_block = door_chunk.get(door_local);
            if current_block == WOODEN_DOOR {
                let state = world.door_states.entry(hit).or_insert(DoorState::Closed);
                *state = match state {
                    DoorState::Open => DoorState::Closed,
                    DoorState::Closed => DoorState::Open,
                };
                survival.notice = format!(
                    "Puerta: {}",
                    match state {
                        DoorState::Open => "Abierta",
                        DoorState::Closed => "Cerrada",
                    }
                );
                survival.notice_elapsed = 2.0;
                return;
            }
            if current_block == FURNACE {
                world.furnaces.entry(hit).or_default();
                crafting_ui.furnace_requested = Some(hit);
                return;
            }
        }
    }

    let target = if remove { hit } else { previous };
    let (chunk_coord, local) = block_address(target);
    let mut mined_block = None;
    {
        let Some(chunk) = world.chunks.get_mut(&chunk_coord) else {
            return;
        };
        let current_block = chunk.get(local);
        if remove {
            if current_block == AIR {
                return;
            }
            chunk.set(local, AIR);
            mined_block = Some(current_block);
            if let Some(drop) = block_definition(current_block).drop {
                spawn_block_drop(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target.as_vec3() + Vec3::splat(0.5),
                    inventory_item_for_block_drop(drop),
                );
            }
            // Spawn particle effects
            spawn_block_particles(
                &mut commands,
                &mut meshes,
                &mut materials,
                target.as_vec3(),
                current_block,
            );
            // Remove door state if removed
            if current_block == WOODEN_DOOR {
                world.door_states.remove(&target);
            }
            // A broken furnace spills whatever it was holding.
            if let Some(furnace) = world.furnaces.remove(&target) {
                for stack in [furnace.input, furnace.fuel, furnace.output]
                    .into_iter()
                    .flatten()
                {
                    for _ in 0..stack.count {
                        spawn_block_drop(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            target.as_vec3() + Vec3::splat(0.5),
                            stack.item,
                        );
                    }
                }
            }
        } else {
            if current_block != AIR
                || !inventory.slots[selected_slot.0]
                    .is_some_and(|stack| stack.item == Item::Block(selected.0))
                || block_overlaps_player(target, transform.translation)
            {
                return;
            }
            item_inventory::take_one(&mut inventory.slots[selected_slot.0]);
            chunk.set(local, selected.0);
            // Initialize door state if placing a door
            if selected.0 == WOODEN_DOOR {
                world.door_states.insert(target, DoorState::Closed);
            }
        }
    }
    if let Some(block) = mined_block {
        survival.experience = survival
            .experience
            .saturating_add(experience_for_block(block));
        if selected_mining_tool(block, inventory.slots[selected_slot.0]).is_some() {
            let durability = wear_selected_tool(&mut inventory, selected_slot.0)
                .expect("selected mining tool exists");
            if let Some(drop) = block_definition(block).drop {
                spawn_block_drop(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target.as_vec3() + Vec3::splat(0.5) + Vec3::Y * 0.15,
                    inventory_item_for_block_drop(drop),
                );
            }
            survival.experience = survival
                .experience
                .saturating_add(experience_for_block(block));
            if durability == 0 {
                survival.notice = "La herramienta se rompió.".into();
                survival.notice_elapsed = 4.0;
            }
        }
        if block == LEAVES
            && (coordinate_hash(target.x, target.z, world.seed) ^ target.y as u64).is_multiple_of(3)
        {
            survival.food = survival.food.saturating_add(1).min(99);
            survival.notice = "Encontraste una baya.".into();
            survival.notice_elapsed = 3.0;
        }
    }
    let updated_chunk = world.chunks[&chunk_coord].blocks.clone();
    world.modified_chunks.insert(
        chunk_coord,
        Chunk {
            blocks: updated_chunk,
        },
    );
    storage.dirty_chunks.insert(chunk_coord);
    storage.failed_chunks.remove(&chunk_coord);
    storage.status = "Cambios pendientes de guardar".into();
    mark_dirty(&mut world, chunk_coord, local);
}

pub(crate) fn selected_mining_tool(block: u8, stack: Option<ItemStack>) -> Option<MiningTool> {
    let kind = match stack?.item {
        Item::Tool { kind, .. } => kind,
        _ => return None,
    };
    match (block, kind) {
        (STONE | COAL_ORE | COPPER_ORE | IRON_ORE | FURNACE, ToolKind::Pickaxe) => {
            Some(MiningTool::Pickaxe)
        }
        (WOOD | LEAVES, ToolKind::Axe) => Some(MiningTool::Axe),
        (SAND | DIRT, ToolKind::Shovel) => Some(MiningTool::Shovel),
        _ => None,
    }
}

pub(crate) fn wear_selected_tool(inventory: &mut Inventory, slot: usize) -> Option<u32> {
    let stack = inventory.slots.get_mut(slot)?.as_mut()?;
    let Item::Tool {
        ref mut durability, ..
    } = stack.item
    else {
        return None;
    };
    *durability -= 1;
    let remaining = *durability;
    if remaining == 0 {
        inventory.slots[slot] = None;
    }
    Some(remaining)
}

/// Material of the held tool; bare hands count as stone, the weakest tier.
pub(crate) fn selected_tool_material(stack: Option<ItemStack>) -> ToolMaterial {
    match stack.map(|stack| stack.item) {
        Some(Item::Tool { material, .. }) => material,
        _ => ToolMaterial::Stone,
    }
}

/// Mining time with a stone tool, or by hand when `tool` is `None`.
#[cfg(test)]
pub(crate) fn mining_duration(block: u8, tool: Option<MiningTool>) -> f32 {
    mining_duration_with(block, tool, ToolMaterial::Stone)
}

pub(crate) fn mining_duration_with(
    block: u8,
    tool: Option<MiningTool>,
    material: ToolMaterial,
) -> f32 {
    let base = block_definition(block).hardness;
    if tool.is_some() {
        base / material.mining_speed()
    } else {
        base
    }
}

pub(crate) fn raycast_with_distance(
    world: &WorldData,
    origin: Vec3,
    direction: Vec3,
    reach: f32,
) -> Option<(IVec3, IVec3, f32)> {
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
    let mut distance = 0.0;

    for _ in 0..256 {
        if block_at(world, cell) != AIR {
            return Some((cell, previous, distance));
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
        distance = t_max[axis];
        cell[axis] += step[axis];
        t_max[axis] += t_delta[axis];
    }
    None
}

pub(crate) fn initial_ray_distance(origin: f32, cell: i32, direction: f32, step: i32) -> f32 {
    if step > 0 {
        ((cell + 1) as f32 - origin) / direction
    } else if step < 0 {
        (origin - cell as f32) / -direction
    } else {
        f32::INFINITY
    }
}

pub(crate) fn ray_delta(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        direction.abs().recip()
    }
}
