//! Player movement, camera look, pointer capture and collision.

use crate::*;

pub(crate) const PLAYER_HALF_WIDTH: f32 = 0.30;

pub(crate) const PLAYER_EYE_HEIGHT: f32 = 1.62;

pub(crate) const PLAYER_HEADROOM: f32 = 0.18;

pub(crate) const WALK_SPEED: f32 = 5.0;

pub(crate) const SPRINT_SPEED: f32 = 8.0;

pub(crate) const GROUND_ACCELERATION: f32 = 28.0;

pub(crate) const GROUND_FRICTION: f32 = 36.0;

pub(crate) const AIR_ACCELERATION: f32 = 6.0;

pub(crate) const AIR_DRAG: f32 = 1.2;

pub(crate) const FLIGHT_ACCELERATION: f32 = 42.0;

pub(crate) const FLIGHT_FRICTION: f32 = 30.0;

pub(crate) const GRAVITY: f32 = 24.0;

pub(crate) const JUMP_SPEED: f32 = 8.5;

pub(crate) const STEP_HEIGHT: f32 = 1.0;

pub(crate) const STEP_UP_SPEED: f32 = 7.0;

pub(crate) const JUMP_BUFFER_SECONDS: f32 = 0.12;

pub(crate) const COYOTE_TIME_SECONDS: f32 = 0.10;

#[derive(Component)]
pub(crate) struct Player {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) velocity: Vec3,
    pub(crate) grounded: bool,
    pub(crate) flying: bool,
    pub(crate) jump_buffer: f32,
    pub(crate) coyote_time: f32,
    pub(crate) step_up_remaining: f32,
    pub(crate) jump_key_down: bool,
    pub(crate) flight_key_down: bool,
}

#[derive(Resource, Default)]
pub(crate) struct PointerState {
    pub(crate) captured_this_frame: bool,
}

pub(crate) fn capture_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    crafting_ui: Res<CraftingUi>,
    chat: Res<ChatState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
) {
    pointer.captured_this_frame = false;
    if crafting_ui.is_active() || chat.open {
        return;
    }
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

pub(crate) fn look(
    mut motion: EventReader<MouseMotion>,
    mut player: Query<(&mut Transform, &mut Player)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pointer: Res<PointerState>,
    chat: Res<ChatState>,
) {
    if chat.open {
        return;
    }
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

pub(crate) fn move_player(
    time: Res<Time<Fixed>>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<WorldData>,
    mut survival: ResMut<Survival>,
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

    let delta = time.delta_secs();
    let jump_pressed = keys.pressed(KeyCode::Space) && !player.jump_key_down;
    let flight_pressed = keys.pressed(KeyCode::KeyF) && !player.flight_key_down;
    player.jump_key_down = keys.pressed(KeyCode::Space);
    player.flight_key_down = keys.pressed(KeyCode::KeyF);

    if flight_pressed {
        player.flying = !player.flying;
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        player.jump_buffer = 0.0;
        player.coyote_time = 0.0;
        player.step_up_remaining = 0.0;
        info!(
            "Movement mode: {}",
            if player.flying { "flight" } else { "walking" }
        );
    }

    if !player.flying {
        let safe_position = find_safe_spawn_position(&world, transform.translation);
        if safe_position != transform.translation {
            transform.translation = safe_position;
            player.velocity = Vec3::ZERO;
            player.grounded = false;
            player.jump_buffer = 0.0;
            player.coyote_time = 0.0;
            player.step_up_remaining = 0.0;
        }
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

    if player.flying {
        if keys.pressed(KeyCode::Space) {
            direction.y += 1.0;
        }
        if keys.pressed(KeyCode::ControlLeft) {
            direction.y -= 1.0;
        }
        let target_velocity = direction.normalize_or_zero() * SPRINT_SPEED;
        let control_rate = if direction == Vec3::ZERO {
            FLIGHT_FRICTION
        } else {
            FLIGHT_ACCELERATION
        };
        player.velocity = approach_velocity(player.velocity, target_velocity, control_rate * delta);
        transform.translation += player.velocity * delta;
        return;
    }

    let swimming = block_at(
        &world,
        (transform.translation - Vec3::Y * 0.9).floor().as_ivec3(),
    ) == WATER;
    let speed = if swimming {
        WALK_SPEED * 0.7
    } else if keys.pressed(KeyCode::ShiftLeft) {
        SPRINT_SPEED
    } else {
        WALK_SPEED
    };
    let input_direction = direction.normalize_or_zero();
    let target_velocity = input_direction * speed;
    let horizontal_velocity = Vec2::new(player.velocity.x, player.velocity.z);
    let target_horizontal_velocity = Vec2::new(target_velocity.x, target_velocity.z);
    let control_rate = if player.grounded {
        if input_direction == Vec3::ZERO {
            GROUND_FRICTION
        } else {
            GROUND_ACCELERATION
        }
    } else if input_direction == Vec3::ZERO {
        AIR_DRAG
    } else {
        AIR_ACCELERATION
    };
    let velocity_delta = target_horizontal_velocity - horizontal_velocity;
    let horizontal_velocity = if velocity_delta.length() <= control_rate * delta {
        target_horizontal_velocity
    } else {
        horizontal_velocity + velocity_delta.normalize() * control_rate * delta
    };
    player.velocity.x = horizontal_velocity.x;
    player.velocity.z = horizontal_velocity.y;

    let was_grounded = player.grounded;
    if was_grounded {
        player.coyote_time = COYOTE_TIME_SECONDS;
    } else {
        player.coyote_time = (player.coyote_time - delta).max(0.0);
    }
    if jump_pressed {
        player.jump_buffer = JUMP_BUFFER_SECONDS;
    } else {
        player.jump_buffer = (player.jump_buffer - delta).max(0.0);
    }

    if swimming {
        player.jump_buffer = 0.0;
        if jump_pressed {
            player.velocity.y = 3.0;
        }
        player.velocity.y = (player.velocity.y - 4.0 * delta).max(-3.0);
    } else {
        if player.jump_buffer > 0.0 && player.coyote_time > 0.0 {
            player.velocity.y = JUMP_SPEED;
            player.jump_buffer = 0.0;
            player.coyote_time = 0.0;
            player.grounded = false;
            player.step_up_remaining = 0.0;
        }
        if player.step_up_remaining > 0.0 {
            player.velocity.y = 0.0;
        } else {
            player.velocity.y = (player.velocity.y - GRAVITY * delta).max(-50.0);
        }
    }

    let displacement = player.velocity * delta;
    let step_count = (displacement.abs().max_element() / 0.05).ceil().max(1.0) as usize;
    let step = displacement / step_count as f32;
    let can_step = was_grounded || player.step_up_remaining > 0.0;
    player.grounded = false;

    for _ in 0..step_count {
        for axis in [0, 2] {
            let mut candidate = transform.translation;
            candidate[axis] += step[axis];
            if !collides_with_world(&world, candidate) {
                transform.translation = candidate;
            } else if can_step && step[axis] != 0.0 && player.step_up_remaining == 0.0 {
                let raised = transform.translation + Vec3::Y * STEP_HEIGHT;
                let mut stepped = raised;
                stepped[axis] += step[axis];
                if !collides_with_world(&world, raised) && !collides_with_world(&world, stepped) {
                    player.step_up_remaining = STEP_HEIGHT;
                    player.velocity.y = 0.0;
                }
            }
        }
    }

    if player.step_up_remaining > 0.0 {
        let lift = (STEP_UP_SPEED * delta).min(player.step_up_remaining);
        let mut candidate = transform.translation;
        candidate.y += lift;
        if !collides_with_world(&world, candidate) {
            transform.translation = candidate;
            player.step_up_remaining -= lift;
            player.grounded = true;
            player.velocity.y = 0.0;
        } else {
            player.step_up_remaining = 0.0;
        }
    } else {
        for _ in 0..step_count {
            let mut candidate = transform.translation;
            candidate.y += step.y;
            if !collides_with_world(&world, candidate) {
                transform.translation = candidate;
            } else {
                if step.y < 0.0 {
                    player.grounded = true;
                    let impact_speed = -player.velocity.y;
                    apply_damage(&mut survival, fall_damage(impact_speed), "Daño por caída");
                }
                player.velocity.y = 0.0;
            }
        }
    }
}

pub(crate) fn approach_velocity(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let difference = target - current;
    if difference.length() <= max_delta {
        target
    } else {
        current + difference.normalize_or_zero() * max_delta
    }
}

pub(crate) fn collides_with_world(world: &WorldData, eye_position: Vec3) -> bool {
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
                if is_block_solid(world, IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn block_overlaps_player(block: IVec3, eye_position: Vec3) -> bool {
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

pub(crate) fn player_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}
