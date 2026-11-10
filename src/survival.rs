//! Health, hunger, food and experience.

use crate::*;

pub(crate) const MAX_HEALTH: f32 = 20.0;

pub(crate) const MAX_HUNGER: f32 = 20.0;

pub(crate) const PASSIVE_HUNGER_INTERVAL: f32 = 75.0;

pub(crate) const SPRINT_HUNGER_INTERVAL: f32 = 30.0;

pub(crate) const STARVATION_DAMAGE_INTERVAL: f32 = 10.0;

pub(crate) const HEALTH_REGEN_INTERVAL: f32 = 8.0;

// Only berry rations are edible so far; the other foods await cooking.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FoodType {
    BerryRation, // saturation: 4
    Apple,       // saturation: 6
    Mushroom,    // saturation: 2 (puede ser venenoso)
    Wheat,       // saturation: 8
    RawMeat,     // saturation: 3
    CookedMeat,  // saturation: 8
}

impl FoodType {
    pub(crate) fn saturation(&self) -> f32 {
        match self {
            FoodType::BerryRation => 4.0,
            FoodType::Apple => 6.0,
            FoodType::Mushroom => 2.0,
            FoodType::Wheat => 8.0,
            FoodType::RawMeat => 3.0,
            FoodType::CookedMeat => 8.0,
        }
    }

    pub(crate) fn is_poisonous(&self) -> bool {
        matches!(self, FoodType::Mushroom)
    }
}

#[derive(Resource)]
pub(crate) struct Survival {
    pub(crate) health: f32,
    pub(crate) hunger: f32,
    pub(crate) experience: u32,
    pub(crate) food: u32,
    pub(crate) passive_hunger_elapsed: f32,
    pub(crate) sprint_hunger_elapsed: f32,
    pub(crate) starvation_elapsed: f32,
    pub(crate) regeneration_elapsed: f32,
    pub(crate) notice: String,
    pub(crate) notice_elapsed: f32,
}

impl Default for Survival {
    fn default() -> Self {
        Self {
            health: MAX_HEALTH,
            hunger: MAX_HUNGER,
            experience: 0,
            food: default_starting_food(),
            passive_hunger_elapsed: 0.0,
            sprint_hunger_elapsed: 0.0,
            starvation_elapsed: 0.0,
            regeneration_elapsed: 0.0,
            notice: String::new(),
            notice_elapsed: 0.0,
        }
    }
}

pub(crate) fn experience_required(level: u32) -> u32 {
    10_u32.saturating_add(level.saturating_mul(5))
}

pub(crate) fn experience_progress(total_experience: u32) -> (u32, u32, u32) {
    let mut remaining = total_experience;
    let mut level = 0;
    loop {
        let required = experience_required(level);
        if remaining < required || level == u32::MAX {
            return (level, remaining, required);
        }
        remaining -= required;
        level += 1;
    }
}

pub(crate) fn experience_for_block(block: u8) -> u32 {
    block_definition(block).experience
}

pub(crate) fn fall_damage(impact_speed: f32) -> f32 {
    if impact_speed <= 12.0 {
        0.0
    } else {
        ((impact_speed - 12.0) / 4.0).floor() + 1.0
    }
}

pub(crate) fn eat_food(survival: &mut Survival) -> bool {
    eat_food_of_type(survival, FoodType::BerryRation)
}

pub(crate) fn eat_food_of_type(survival: &mut Survival, food_type: FoodType) -> bool {
    if survival.food == 0 || survival.hunger >= MAX_HUNGER {
        return false;
    }

    survival.food -= 1;
    let saturation = food_type.saturation();
    survival.hunger = (survival.hunger + saturation).min(MAX_HUNGER);

    let food_name = match food_type {
        FoodType::BerryRation => "Comiste una ración de bayas.",
        FoodType::Apple => "Comiste una manzana.",
        FoodType::Mushroom => "Comiste un hongo.",
        FoodType::Wheat => "Comiste trigo.",
        FoodType::RawMeat => "Comiste carne cruda.",
        FoodType::CookedMeat => "Comiste carne asada.",
    };

    survival.notice = food_name.into();
    survival.notice_elapsed = 3.0;

    if food_type.is_poisonous() {
        apply_damage(survival, 2.0, "Envenenamiento");
    }

    true
}

pub(crate) fn advance_survival_stats(survival: &mut Survival, delta: f32, sprinting: bool) {
    survival.passive_hunger_elapsed += delta;
    while survival.passive_hunger_elapsed >= PASSIVE_HUNGER_INTERVAL {
        survival.passive_hunger_elapsed -= PASSIVE_HUNGER_INTERVAL;
        survival.hunger = (survival.hunger - 1.0).max(0.0);
    }

    if sprinting {
        survival.sprint_hunger_elapsed += delta;
        while survival.sprint_hunger_elapsed >= SPRINT_HUNGER_INTERVAL {
            survival.sprint_hunger_elapsed -= SPRINT_HUNGER_INTERVAL;
            survival.hunger = (survival.hunger - 1.0).max(0.0);
        }
    } else {
        survival.sprint_hunger_elapsed = 0.0;
    }

    if survival.hunger >= 18.0 && survival.health < MAX_HEALTH {
        survival.regeneration_elapsed += delta;
        while survival.regeneration_elapsed >= HEALTH_REGEN_INTERVAL
            && survival.hunger >= 18.0
            && survival.health < MAX_HEALTH
        {
            survival.regeneration_elapsed -= HEALTH_REGEN_INTERVAL;
            survival.health = (survival.health + 1.0).min(MAX_HEALTH);
            survival.hunger = (survival.hunger - 0.5).max(0.0);
        }
    } else {
        survival.regeneration_elapsed = 0.0;
    }

    if survival.hunger == 0.0 {
        survival.starvation_elapsed += delta;
        while survival.starvation_elapsed >= STARVATION_DAMAGE_INTERVAL && survival.health > 0.0 {
            survival.starvation_elapsed -= STARVATION_DAMAGE_INTERVAL;
            apply_damage(survival, 1.0, "Daño por hambre");
        }
    } else {
        survival.starvation_elapsed = 0.0;
    }
}

pub(crate) fn apply_damage(survival: &mut Survival, amount: f32, notice: &str) {
    if amount <= 0.0 || survival.health <= 0.0 {
        return;
    }
    survival.health = (survival.health - amount).max(0.0);
    survival.notice = notice.to_string();
    survival.notice_elapsed = 4.0;
}

pub(crate) fn update_survival(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<WorldData>,
    mut survival: ResMut<Survival>,
    mut player: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = player.get_single_mut() else {
        return;
    };
    let delta = time.delta_secs();
    survival.notice_elapsed = (survival.notice_elapsed - delta).max(0.0);
    if survival.notice_elapsed == 0.0 {
        survival.notice.clear();
    }

    let moving = player.velocity.xz().length_squared() > 0.1;
    let sprinting = !player.flying && moving && keys.pressed(KeyCode::ShiftLeft);
    advance_survival_stats(&mut survival, delta, sprinting);

    if survival.health <= 0.0 {
        let (spawn_x, ground, spawn_z) = world_spawn_point(world.seed, world.terrain_version);
        transform.translation =
            Vec3::new(spawn_x, ground as f32 + 1.0 + PLAYER_EYE_HEIGHT, spawn_z);
        player.velocity = Vec3::ZERO;
        player.grounded = false;
        player.flying = false;
        player.jump_buffer = 0.0;
        player.coyote_time = 0.0;
        player.step_up_remaining = 0.0;
        survival.health = MAX_HEALTH / 2.0;
        survival.hunger = MAX_HUNGER / 2.0;
        survival.passive_hunger_elapsed = 0.0;
        survival.sprint_hunger_elapsed = 0.0;
        survival.starvation_elapsed = 0.0;
        survival.regeneration_elapsed = 0.0;
        survival.notice = "Has reaparecido; conservas inventario y experiencia.".into();
        survival.notice_elapsed = 6.0;
    }
}

pub(crate) fn consume_stored_food(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut survival: ResMut<Survival>,
) {
    if keys.just_pressed(KeyCode::KeyR)
        && windows
            .get_single()
            .is_ok_and(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
    {
        eat_food(&mut survival);
    }
}
