//! Inventory and crafting screens.

use crate::*;

#[derive(Component)]
pub(crate) struct InventoryPanel;

#[derive(Component, Clone, Copy)]
pub(crate) enum InventoryTarget {
    Slot(usize),
    Small(usize),
    Large(usize),
    SmallResult,
    LargeResult,
    FurnaceInput,
    FurnaceFuel,
    FurnaceOutput,
}

/// Which screen the shared inventory panel is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelMode {
    Inventory,
    Table,
    Furnace,
}

#[derive(Component)]
pub(crate) struct FurnaceStatusLabel;

#[derive(Component)]
pub(crate) struct InventoryLabel(pub(crate) InventoryTarget);

#[derive(Component)]
pub(crate) struct InventoryCursorLabel;

#[derive(Component)]
pub(crate) struct InventoryGridMode(pub(crate) PanelMode);

#[derive(Resource, Default)]
pub(crate) struct CraftingUi {
    pub(crate) open: bool,
    pub(crate) just_closed: bool,
    pub(crate) inventory_open: bool,
    /// Position of the furnace whose screen is open.
    pub(crate) furnace: Option<IVec3>,
    /// Set when the player uses a furnace; the screen opens next frame.
    pub(crate) furnace_requested: Option<IVec3>,
}

impl CraftingUi {
    /// True while any inventory screen has the cursor.
    pub(crate) fn is_active(&self) -> bool {
        self.open || self.inventory_open || self.furnace.is_some()
    }

    fn mode(&self) -> Option<PanelMode> {
        if self.furnace.is_some() {
            Some(PanelMode::Furnace)
        } else if self.open {
            Some(PanelMode::Table)
        } else if self.inventory_open {
            Some(PanelMode::Inventory)
        } else {
            None
        }
    }
}

pub(crate) fn block_type_name(block_type: u8) -> &'static str {
    block_definition(block_type).name_es
}

pub(crate) fn item_name(item: Item) -> &'static str {
    match item {
        Item::Block(block) => block_type_name(block),
        _ => item.definition().name,
    }
}

pub(crate) fn spawn_inventory_slot(parent: &mut ChildBuilder, target: InventoryTarget) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(72.0),
                height: Val::Px(52.0),
                border: UiRect::all(Val::Px(1.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.17, 0.19)),
            BorderColor(Color::srgb(0.55, 0.55, 0.55)),
            target,
        ))
        .with_child((
            Text::new(""),
            TextFont::from_font_size(12.0),
            TextColor(Color::WHITE),
            InventoryLabel(target),
        ));
}

pub(crate) fn setup_slot_inventory_ui(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(100),
            InventoryPanel,
            GameEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(750.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.12, 0.15)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Inventario [E] | Mesa 3x3 [C] | Horno: clic derecho | Cerrar [Esc]"),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Izquierdo: mover / combinar | Derecho: dividir / colocar uno"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Recetas: consulta la guia en README.md"),
                    TextFont::from_font_size(13.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Cursor: vacio"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                    InventoryCursorLabel,
                ));
                panel
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        InventoryGridMode(PanelMode::Furnace),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|column| {
                            column.spawn((
                                Text::new("Material"),
                                TextFont::from_font_size(12.0),
                                TextColor(Color::WHITE),
                            ));
                            spawn_inventory_slot(column, InventoryTarget::FurnaceInput);
                            column.spawn((
                                Text::new("Combustible"),
                                TextFont::from_font_size(12.0),
                                TextColor(Color::WHITE),
                            ));
                            spawn_inventory_slot(column, InventoryTarget::FurnaceFuel);
                        });
                        row.spawn((
                            Text::new("->"),
                            TextFont::from_font_size(24.0),
                            TextColor(Color::WHITE),
                        ));
                        spawn_inventory_slot(row, InventoryTarget::FurnaceOutput);
                        row.spawn((
                            Text::new(""),
                            TextFont::from_font_size(14.0),
                            TextColor(Color::WHITE),
                            FurnaceStatusLabel,
                        ));
                    });
                for (large, width) in [(false, 2), (true, 3)] {
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(12.0),
                                ..default()
                            },
                            InventoryGridMode(if large {
                                PanelMode::Table
                            } else {
                                PanelMode::Inventory
                            }),
                        ))
                        .with_children(|row| {
                            row.spawn(Node {
                                width: Val::Px(width as f32 * 76.0),
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(4.0),
                                row_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|grid| {
                                for index in 0..width * width {
                                    spawn_inventory_slot(
                                        grid,
                                        if large {
                                            InventoryTarget::Large(index)
                                        } else {
                                            InventoryTarget::Small(index)
                                        },
                                    );
                                }
                            });
                            row.spawn((
                                Text::new("->"),
                                TextFont::from_font_size(24.0),
                                TextColor(Color::WHITE),
                            ));
                            spawn_inventory_slot(
                                row,
                                if large {
                                    InventoryTarget::LargeResult
                                } else {
                                    InventoryTarget::SmallResult
                                },
                            );
                        });
                }
                panel.spawn((
                    Text::new("Espacios 1-9: barra rapida"),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::WHITE),
                ));
                panel
                    .spawn(Node {
                        width: Val::Px(684.0),
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(4.0),
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|grid| {
                        for index in 0..INVENTORY_SIZE {
                            spawn_inventory_slot(grid, InventoryTarget::Slot(index));
                        }
                    });
            });
        });
}

pub(crate) fn inventory_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<CraftingUi>,
    chat: Res<ChatState>,
    mut panel: Query<&mut Node, With<InventoryPanel>>,
    mut modes: Query<(&InventoryGridMode, &mut Node), Without<InventoryPanel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    ui.just_closed = false;
    if chat.open {
        ui.furnace_requested = None;
        return;
    }
    let active = ui.is_active();
    let mode = if let Some(position) = ui.furnace_requested.take() {
        ui.furnace = Some(position);
        Some(PanelMode::Furnace)
    } else if keys.just_pressed(KeyCode::Escape) && active {
        None
    } else if keys.just_pressed(KeyCode::KeyE) {
        // E closes whichever screen is open, or opens the inventory.
        (!active).then_some(PanelMode::Inventory)
    } else if keys.just_pressed(KeyCode::KeyC) {
        (!ui.open).then_some(PanelMode::Table)
    } else {
        return;
    };
    let Ok(mut window) = windows.get_single_mut() else {
        error!("Primary window is missing while toggling inventory");
        return;
    };
    let Ok(mut root) = panel.get_single_mut() else {
        error!("Inventory panel is missing");
        return;
    };
    ui.open = mode == Some(PanelMode::Table);
    ui.inventory_open = mode == Some(PanelMode::Inventory);
    if mode != Some(PanelMode::Furnace) {
        ui.furnace = None;
    }
    ui.just_closed = active && mode.is_none();
    root.display = if mode.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for (grid, mut node) in &mut modes {
        node.display = if mode == Some(grid.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    window.cursor_options.grab_mode = if mode.is_some() {
        CursorGrabMode::None
    } else {
        CursorGrabMode::Locked
    };
    window.cursor_options.visible = mode.is_some();
}

/// Takes smelted items out of a furnace; nothing can be put in this slot.
fn take_output(output: &mut Option<ItemStack>, cursor: &mut Option<ItemStack>) {
    let Some(stack) = *output else {
        return;
    };
    match cursor {
        None => *cursor = output.take(),
        Some(held)
            if held.item == stack.item && held.count + stack.count <= held.item.stack_limit() =>
        {
            held.count += stack.count;
            *output = None;
        }
        Some(_) => {}
    }
}

pub(crate) fn inventory_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    ui: Res<CraftingUi>,
    mut inventory: ResMut<Inventory>,
    mut world: ResMut<WorldData>,
    buttons: Query<(&Interaction, &InventoryTarget)>,
) {
    let Some(mode) = ui.mode() else {
        return;
    };
    let right = mouse.just_pressed(MouseButton::Right);
    if !right && !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((_, target)) = buttons.iter().find(|(interaction, target)| {
        **interaction != Interaction::None
            && match target {
                InventoryTarget::Small(_) | InventoryTarget::SmallResult => {
                    mode == PanelMode::Inventory
                }
                InventoryTarget::Large(_) | InventoryTarget::LargeResult => {
                    mode == PanelMode::Table
                }
                InventoryTarget::FurnaceInput
                | InventoryTarget::FurnaceFuel
                | InventoryTarget::FurnaceOutput => mode == PanelMode::Furnace,
                InventoryTarget::Slot(_) => true,
            }
    }) else {
        return;
    };
    let inventory = &mut *inventory;
    match *target {
        InventoryTarget::Slot(index) => {
            item_inventory::click_slot(&mut inventory.slots[index], &mut inventory.cursor, right)
        }
        InventoryTarget::Small(index) => item_inventory::click_slot(
            &mut inventory.small_grid[index],
            &mut inventory.cursor,
            right,
        ),
        InventoryTarget::Large(index) => item_inventory::click_slot(
            &mut inventory.large_grid[index],
            &mut inventory.cursor,
            right,
        ),
        InventoryTarget::SmallResult => {
            item_inventory::craft(&mut inventory.small_grid, 2, &mut inventory.cursor);
        }
        InventoryTarget::LargeResult => {
            item_inventory::craft(&mut inventory.large_grid, 3, &mut inventory.cursor);
        }
        InventoryTarget::FurnaceInput
        | InventoryTarget::FurnaceFuel
        | InventoryTarget::FurnaceOutput => {
            let Some(furnace) = ui
                .furnace
                .and_then(|position| world.furnaces.get_mut(&position))
            else {
                return;
            };
            match *target {
                InventoryTarget::FurnaceInput => {
                    item_inventory::click_slot(&mut furnace.input, &mut inventory.cursor, right)
                }
                InventoryTarget::FurnaceFuel => {
                    item_inventory::click_slot(&mut furnace.fuel, &mut inventory.cursor, right)
                }
                _ => take_output(&mut furnace.output, &mut inventory.cursor),
            }
        }
    }
}

pub(crate) fn update_inventory_ui(
    inventory: Res<Inventory>,
    ui: Res<CraftingUi>,
    world: Res<WorldData>,
    mut labels: Query<
        (&InventoryLabel, &mut Text),
        (Without<InventoryCursorLabel>, Without<FurnaceStatusLabel>),
    >,
    mut cursor: Query<&mut Text, (With<InventoryCursorLabel>, Without<FurnaceStatusLabel>)>,
    mut status: Query<&mut Text, With<FurnaceStatusLabel>>,
) {
    let furnace = ui
        .furnace
        .and_then(|position| world.furnaces.get(&position));
    // An open furnace keeps smelting, so its screen refreshes every frame.
    if !inventory.is_changed() && furnace.is_none() {
        return;
    }
    for (label, mut text) in &mut labels {
        let stack = match label.0 {
            InventoryTarget::Slot(index) => inventory.slots[index],
            InventoryTarget::Small(index) => inventory.small_grid[index],
            InventoryTarget::Large(index) => inventory.large_grid[index],
            InventoryTarget::SmallResult => item_inventory::recipe(&inventory.small_grid, 2),
            InventoryTarget::LargeResult => item_inventory::recipe(&inventory.large_grid, 3),
            InventoryTarget::FurnaceInput => furnace.and_then(|furnace| furnace.input),
            InventoryTarget::FurnaceFuel => furnace.and_then(|furnace| furnace.fuel),
            InventoryTarget::FurnaceOutput => furnace.and_then(|furnace| furnace.output),
        };
        let label = stack.map_or_else(String::new, |stack| {
            format!("{}\nx{}", item_name(stack.item), stack.count)
        });
        if text.0 != label {
            text.0 = label;
        }
    }
    for mut text in &mut cursor {
        let label = inventory.cursor.map_or_else(
            || "Cursor: vacio".into(),
            |stack| format!("Cursor: {} x{}", item_name(stack.item), stack.count),
        );
        if text.0 != label {
            text.0 = label;
        }
    }
    if let Some(furnace) = furnace {
        let label = furnace_status(furnace);
        for mut text in &mut status {
            if text.0 != label {
                text.0.clone_from(&label);
            }
        }
    }
}

pub(crate) fn furnace_status(furnace: &Furnace) -> String {
    let fire = if furnace.is_lit() {
        format!("Fuego: {:.0} s", furnace.burn_remaining.ceil())
    } else {
        "Apagado".into()
    };
    let progress = (furnace.progress / furnace::SMELT_SECONDS * 100.0).round();
    format!("{fire}\nProgreso: {progress:.0} %")
}

/// Smelting continues everywhere, including furnaces in unloaded chunks.
pub(crate) fn tick_furnaces(time: Res<Time>, mut world: ResMut<WorldData>) {
    if world.furnaces.is_empty() {
        return;
    }
    let seconds = time.delta_secs();
    for furnace in world.furnaces.values_mut() {
        furnace.tick(seconds);
    }
}
