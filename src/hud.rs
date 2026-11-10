//! In-game HUD: hotbar, survival bars, block selection and chat.

use crate::*;

pub(crate) const HOTBAR_SLOT_COUNT: usize = HOTBAR_SIZE;

#[derive(Component)]
pub(crate) struct HudRoot;

#[derive(Component)]
pub(crate) struct ChatBar;

#[derive(Component)]
pub(crate) struct ChatInputText;

#[derive(Resource)]
pub(crate) struct SelectedBlock(pub(crate) u8);

impl Default for SelectedBlock {
    fn default() -> Self {
        Self(GRASS)
    }
}

#[derive(Resource, Default)]
pub(crate) struct SelectedHotbarSlot(pub(crate) usize);

#[derive(Component)]
pub(crate) struct ExperienceText;

#[derive(Component, Clone, Copy)]
pub(crate) struct SurvivalIcon {
    pub(crate) kind: SurvivalIconKind,
    pub(crate) index: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum SurvivalIconKind {
    Health,
    Hunger,
}

#[derive(Component)]
pub(crate) struct ExperienceFill;

#[derive(Resource, Default)]
pub(crate) struct ChatState {
    pub(crate) open: bool,
    pub(crate) input: String,
}

#[derive(Component)]
pub(crate) struct HotbarSlot(pub(crate) usize);

#[derive(Component)]
pub(crate) struct HotbarCount(pub(crate) usize);

pub(crate) fn setup_hud(commands: &mut Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert((HudRoot, GameEntity))
        .with_children(|root| {
            root.spawn((
                Text::new("+"),
                TextFont::from_font_size(26.0),
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
            ));

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    bottom: Val::Px(154.0),
                    width: Val::Px(620.0),
                    min_height: Val::Px(42.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    align_items: AlignItems::Center,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.78)),
                GlobalZIndex(50),
                ChatBar,
            ))
            .with_child((
                Text::new(""),
                TextFont::from_font_size(18.0),
                TextColor(Color::WHITE),
                ChatInputText,
            ));

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(92.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|hud| {
                hud.spawn(Node {
                    width: Val::Px(548.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|center| {
                    center
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            for (kind, symbol, color) in
                                [(SurvivalIconKind::Health, "♥", Color::srgb(0.92, 0.16, 0.18))]
                            {
                                row.spawn(Node {
                                    width: Val::Px(218.0),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|icons| {
                                    for index in 0..10 {
                                        icons.spawn((
                                            Text::new(symbol),
                                            TextFont::from_font_size(21.0),
                                            TextColor(color),
                                            SurvivalIcon { kind, index },
                                        ));
                                    }
                                });
                            }

                            row.spawn(Node {
                                width: Val::Px(218.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|icons| {
                                for index in 0..10 {
                                    icons.spawn((
                                        Text::new("●"),
                                        TextFont::from_font_size(19.0),
                                        TextColor(Color::srgb(0.86, 0.58, 0.20)),
                                        SurvivalIcon {
                                            kind: SurvivalIconKind::Hunger,
                                            index,
                                        },
                                    ));
                                }
                            });
                        });

                    center.spawn((
                        Text::new("0"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgb(0.45, 1.0, 0.30)),
                        ExperienceText,
                    ));

                    center
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(6.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.05, 0.08, 0.04, 0.95)),
                        ))
                        .with_child((
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.90, 0.12)),
                            ExperienceFill,
                        ));
                });
            });

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)),
            ))
            .with_children(|hotbar| {
                for slot_index in 0..HOTBAR_SLOT_COUNT {
                    hotbar
                        .spawn((
                            Node {
                                width: Val::Px(52.0),
                                height: Val::Px(52.0),
                                border: UiRect::all(Val::Px(2.0)),
                                align_items: AlignItems::End,
                                justify_content: JustifyContent::Start,
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 0.9)),
                            BorderColor(if slot_index == 0 {
                                Color::WHITE
                            } else {
                                Color::srgba(0.12, 0.14, 0.18, 0.9)
                            }),
                            HotbarSlot(slot_index),
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(4.0),
                                    top: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new((slot_index + 1).to_string()),
                                TextFont::from_font_size(12.0),
                                TextColor(Color::WHITE),
                            ));
                            slot.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(4.0),
                                    bottom: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new(""),
                                TextFont::from_font_size(10.0),
                                TextColor(Color::WHITE),
                                HotbarCount(slot_index),
                            ));
                        });
                }
            });
        });
}

pub(crate) fn chat_input(
    mut events: EventReader<KeyboardInput>,
    mut chat: ResMut<ChatState>,
    crafting_ui: Res<CraftingUi>,
    mut cycle: ResMut<DayNightCycle>,
    mut survival: ResMut<Survival>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let opens_chat = matches!(
            &event.logical_key,
            Key::Character(character) if character.eq_ignore_ascii_case("t")
        );
        if !chat.open {
            if opens_chat && !crafting_ui.is_active() {
                chat.open = true;
                chat.input.clear();
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::None;
                    window.cursor_options.visible = true;
                }
            }
            continue;
        }

        match &event.logical_key {
            Key::Escape => {
                chat.open = false;
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
            }
            Key::Enter => {
                let command = chat.input.trim().to_ascii_lowercase();
                match command.as_str() {
                    "/timesetday" => {
                        cycle.time_of_day_seconds = 12.0 * 60.0 * 60.0;
                        survival.notice = "Hora establecida: día.".into();
                        survival.notice_elapsed = 3.0;
                    }
                    "" => {}
                    _ if !command.starts_with('/') => {
                        survival.notice = "Los comandos empiezan con /.".into();
                        survival.notice_elapsed = 3.0;
                    }
                    _ => {
                        survival.notice = "Comando desconocido.".into();
                        survival.notice_elapsed = 3.0;
                    }
                }
                chat.open = false;
                chat.input.clear();
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
            }
            Key::Backspace => {
                chat.input.pop();
            }
            Key::Character(character)
                if chat.input.chars().count() < 128 && !character.chars().any(char::is_control) =>
            {
                chat.input.push_str(character);
            }
            _ => {}
        }
    }
}

pub(crate) fn update_chat_bar(
    chat: Res<ChatState>,
    mut bar: Query<&mut Node, With<ChatBar>>,
    mut input: Query<&mut Text, With<ChatInputText>>,
) {
    if let Ok(mut bar) = bar.get_single_mut() {
        bar.display = if chat.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut input) = input.get_single_mut() {
        input.0 = format!("> {}", chat.input);
    }
}

pub(crate) fn select_block(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
    ui: Res<CraftingUi>,
    chat: Res<ChatState>,
) {
    if ui.is_active() || chat.open {
        return;
    }
    let slot = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .position(|key| keys.just_pressed(*key));
    if let Some(slot) = slot {
        selected_slot.0 = slot;
    }
}

pub(crate) fn select_hotbar(
    mut wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut selected_slot: ResMut<SelectedHotbarSlot>,
) {
    let mut steps = 0_isize;
    for event in wheel.read() {
        steps += mouse_wheel_steps(event.y, event.unit);
    }
    if steps == 0
        || !windows
            .get_single()
            .is_ok_and(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
    {
        return;
    }
    let slot = cycle_hotbar_selection(selected_slot.0, steps);
    selected_slot.0 = slot;
}

pub(crate) fn sync_selected_block(
    inventory: Res<Inventory>,
    selected_slot: Res<SelectedHotbarSlot>,
    mut selected: ResMut<SelectedBlock>,
) {
    selected.0 = match inventory.slots[selected_slot.0] {
        Some(ItemStack {
            item: Item::Block(block),
            ..
        }) => block,
        _ => AIR,
    };
}

pub(crate) fn cycle_hotbar_selection(current: usize, steps: isize) -> usize {
    (current as isize + steps).rem_euclid(HOTBAR_SLOT_COUNT as isize) as usize
}

pub(crate) fn mouse_wheel_steps(y: f32, unit: MouseScrollUnit) -> isize {
    if y == 0.0 {
        return 0;
    }
    let magnitude = match unit {
        MouseScrollUnit::Line => y.abs().ceil(),
        MouseScrollUnit::Pixel => (y.abs() / 40.0).ceil(),
    }
    .clamp(1.0, HOTBAR_SLOT_COUNT as f32) as isize;
    -magnitude * y.signum() as isize
}

pub(crate) fn update_survival_hud(
    survival: Res<Survival>,
    mut icons: Query<(&SurvivalIcon, &mut Text, &mut TextColor), Without<ExperienceText>>,
    mut experience_fill: Query<&mut Node, With<ExperienceFill>>,
    mut experience_text: Query<&mut Text, (With<ExperienceText>, Without<SurvivalIcon>)>,
) {
    let (level, progress, required) = experience_progress(survival.experience);
    for (icon, mut text, mut color) in &mut icons {
        let amount = match icon.kind {
            SurvivalIconKind::Health => survival.health,
            SurvivalIconKind::Hunger => survival.hunger,
        };
        let fill = (amount - icon.index as f32 * 2.0).clamp(0.0, 2.0);
        let half = fill >= 1.0 && fill < 2.0;
        let full = fill >= 2.0;
        let (symbol, tint) = match (icon.kind, full, half) {
            (SurvivalIconKind::Health, true, _) => ("♥", Color::srgb(0.92, 0.16, 0.18)),
            (SurvivalIconKind::Health, false, true) => ("♥", Color::srgb(1.0, 0.46, 0.42)),
            (SurvivalIconKind::Health, false, false) => ("♡", Color::srgb(0.34, 0.18, 0.19)),
            (SurvivalIconKind::Hunger, true, _) => ("●", Color::srgb(0.88, 0.58, 0.18)),
            (SurvivalIconKind::Hunger, false, true) => ("◐", Color::srgb(0.88, 0.58, 0.18)),
            (SurvivalIconKind::Hunger, false, false) => ("○", Color::srgb(0.32, 0.29, 0.25)),
        };
        text.0 = symbol.into();
        color.0 = tint;
    }
    if let Ok(mut fill) = experience_fill.get_single_mut() {
        fill.width = Val::Percent(progress as f32 / required as f32 * 100.0);
    }
    if let Ok(mut text) = experience_text.get_single_mut() {
        text.0 = format!("{level}");
    }
}

#[cfg(test)]
pub(crate) fn format_time_of_day(time_of_day_seconds: f32) -> String {
    let total_minutes = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / 60.0).floor() as u32;
    format!("{:02}:{:02}", (total_minutes / 60) % 24, total_minutes % 60)
}

pub(crate) fn update_hotbar(
    selected: Res<SelectedHotbarSlot>,
    inventory: Res<Inventory>,
    mut slots: Query<(
        &HotbarSlot,
        &mut BorderColor,
        &mut Node,
        &mut BackgroundColor,
    )>,
    mut counts: Query<(&HotbarCount, &mut Text)>,
) {
    let selection_changed = selected.is_changed();
    let inventory_changed = inventory.is_changed();
    if !selection_changed && !inventory_changed {
        return;
    }

    for (slot, mut border_color, mut node, mut background) in &mut slots {
        if selection_changed {
            let is_selected = slot.0 == selected.0;
            border_color.0 = if is_selected {
                Color::WHITE
            } else {
                Color::srgba(0.12, 0.14, 0.18, 0.9)
            };
            node.border = UiRect::all(Val::Px(if is_selected { 3.0 } else { 1.0 }));
        }
        if inventory_changed {
            background.0 = match inventory.slots[slot.0].map(|stack| stack.item) {
                Some(Item::Block(GRASS)) => Color::srgb(0.25, 0.62, 0.22),
                Some(Item::Block(DIRT)) => Color::srgb(0.43, 0.30, 0.18),
                Some(Item::Block(STONE)) => Color::srgb(0.48, 0.50, 0.53),
                Some(Item::Block(SAND)) => Color::srgb(0.82, 0.72, 0.45),
                Some(Item::Block(WOOD)) => Color::srgb(0.48, 0.29, 0.12),
                Some(Item::Block(LEAVES)) => Color::srgb(0.16, 0.48, 0.19),
                Some(_) => Color::srgb(0.28, 0.30, 0.34),
                None => Color::srgba(0.12, 0.14, 0.18, 0.9),
            };
        }
    }
    if inventory_changed {
        for (slot, mut text) in &mut counts {
            text.0 = inventory.slots[slot.0].map_or_else(String::new, |stack| {
                let name = match stack.item {
                    Item::Block(WOODEN_DOOR) => "Puerta",
                    Item::Block(STAIRS) => "Escal.",
                    Item::Block(BRICKS) => "Ladr.",
                    Item::Block(PLANKS) => "Tablon",
                    Item::Block(MUSHROOM) => "Seta",
                    Item::Block(TORCH) => "Antor.",
                    item => item_name(item),
                };
                format!("{name}\n{}", stack.count)
            });
        }
    }
}
