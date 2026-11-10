//! Main menu, world selection and creation, loading and pause screens.

use crate::*;

#[derive(Component)]
pub(crate) struct MenuRoot;

#[derive(Component)]
pub(crate) struct NameInputText;

#[derive(Component)]
pub(crate) struct SeedInputText;

#[derive(Component)]
pub(crate) struct FormErrorText;

#[derive(Component)]
pub(crate) struct LoadingText;

#[derive(Component)]
pub(crate) struct PauseStatus;

#[derive(Component)]
pub(crate) struct PauseRoot;

#[derive(Component)]
pub(crate) struct MenuAction(pub(crate) MenuCommand);

#[derive(Clone, Copy)]
pub(crate) enum MenuCommand {
    Play,
    PlaySelected,
    Create,
    Back,
    DeleteSelected,
    ConfirmDelete,
    CancelDelete,
    StartWorld,
    Resume,
    SaveAndMenu,
    CancelLoading,
    Exit,
}

#[derive(Component)]
pub(crate) struct SelectWorldAction {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
}

#[derive(Component)]
pub(crate) struct WorldSelectionStatus;

#[derive(Component)]
pub(crate) struct FormField(pub(crate) WorldField);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorldField {
    Name,
    Seed,
}

#[derive(Resource)]
pub(crate) struct WorldForm {
    pub(crate) name: String,
    pub(crate) seed: String,
    pub(crate) focus: WorldField,
    pub(crate) error: Option<String>,
}

impl Default for WorldForm {
    fn default() -> Self {
        Self {
            name: "Mi mundo".into(),
            seed: new_world_seed().to_string(),
            focus: WorldField::Name,
            error: None,
        }
    }
}

pub(crate) fn new_world_seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    timestamp
        ^ (u64::from(std::process::id()).rotate_left(23))
        ^ SEQUENCE
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[derive(Resource, Default)]
pub(crate) struct SelectedWorld {
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: Option<String>,
}

pub(crate) fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("THE CUBES"),
                TextFont::from_font_size(54.0),
                TextColor(Color::srgb(0.55, 0.9, 0.72)),
            ));
            root.spawn((
                Text::new("Un mundo de bloques, a tu manera"),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ));
            for (label, action) in [
                ("Jugar", MenuCommand::Play),
                ("Crear mundo", MenuCommand::Create),
                ("Salir", MenuCommand::Exit),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(54.0),
                        margin: UiRect::top(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(22.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

pub(crate) fn setup_world_select(mut commands: Commands, mut selected: ResMut<SelectedWorld>) {
    let worlds = match list_saved_worlds() {
        Ok(worlds) => worlds,
        Err(error) => {
            error!("Could not list saved worlds: {error}");
            Vec::new()
        }
    };

    if selected
        .path
        .as_ref()
        .is_some_and(|path| !worlds.iter().any(|(_, world_path)| world_path == path))
    {
        *selected = SelectedWorld::default();
    }

    let selected_label = selected
        .name
        .as_ref()
        .map(|name| format!("Seleccionado: {name}"))
        .unwrap_or_else(|| "Selecciona un mundo".to_string());

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Elegir mundo"),
                TextFont::from_font_size(36.0),
                TextColor(Color::WHITE),
            ));
            if worlds.is_empty() {
                root.spawn((
                    Text::new("Aún no hay mundos guardados."),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::srgb(0.8, 0.82, 0.86)),
                ));
            }
            for (name, path) in worlds {
                let is_selected = selected.path.as_ref() == Some(&path);
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(360.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(if is_selected {
                        Color::srgb(0.18, 0.36, 0.29)
                    } else {
                        Color::srgb(0.12, 0.2, 0.25)
                    }),
                    SelectWorldAction {
                        path,
                        name: name.clone(),
                    },
                ))
                .with_child((
                    Text::new(name),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
            root.spawn((
                Text::new(selected_label),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.72, 0.86, 0.8)),
                WorldSelectionStatus,
            ));
            for (label, action) in [
                ("Jugar al mundo seleccionado", MenuCommand::PlaySelected),
                ("Crear nuevo mundo", MenuCommand::Create),
                ("Eliminar mundo", MenuCommand::DeleteSelected),
                ("Cancelar", MenuCommand::Back),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(360.0),
                        height: Val::Px(44.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(18.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

pub(crate) fn setup_delete_world_confirmation(
    mut commands: Commands,
    selected: Res<SelectedWorld>,
) {
    let name = selected.name.as_deref().unwrap_or("mundo seleccionado");
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("¿Eliminar el mundo ‘{name}’ permanentemente?")),
                TextFont::from_font_size(24.0),
                TextColor(Color::WHITE),
            ));
            for (label, action) in [
                ("Eliminar", MenuCommand::ConfirmDelete),
                ("Cancelar", MenuCommand::CancelDelete),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(48.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

pub(crate) fn setup_create_world(mut commands: Commands, form: Res<WorldForm>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(520.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.1, 0.14)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Crear mundo"),
                    TextFont::from_font_size(34.0),
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Nombre (hasta 32 caracteres)"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.78, 0.82, 0.88)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            min_height: Val::Px(44.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.16, 0.2)),
                        FormField(WorldField::Name),
                    ))
                    .with_child((
                        Text::new(if form.focus == WorldField::Name {
                            format!("{}_", form.name)
                        } else {
                            form.name.clone()
                        }),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::WHITE),
                        NameInputText,
                    ));
                panel.spawn((
                    Text::new("Semilla (número; la misma semilla repite el terreno)"),
                    TextFont::from_font_size(17.0),
                    TextColor(Color::srgb(0.78, 0.82, 0.88)),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            min_height: Val::Px(44.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.16, 0.2)),
                        FormField(WorldField::Seed),
                    ))
                    .with_child((
                        Text::new(if form.focus == WorldField::Seed {
                            format!("{}_", form.seed)
                        } else {
                            form.seed.clone()
                        }),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::WHITE),
                        SeedInputText,
                    ));
                panel.spawn((
                    Text::new(form.error.clone().unwrap_or_default()),
                    TextFont::from_font_size(16.0),
                    TextColor(Color::srgb(1.0, 0.45, 0.4)),
                    FormErrorText,
                ));
                for (label, action) in [
                    ("Crear y jugar", MenuCommand::StartWorld),
                    ("Cancelar", MenuCommand::Back),
                ] {
                    panel
                        .spawn((
                            Button,
                            Node {
                                height: Val::Px(48.0),
                                margin: UiRect::top(Val::Px(6.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                            MenuAction(action),
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont::from_font_size(19.0),
                            TextColor(Color::WHITE),
                        ));
                }
                panel.spawn((
                    Text::new("Enter cambia el campo activo; Backspace borra."),
                    TextFont::from_font_size(14.0),
                    TextColor(Color::srgb(0.68, 0.72, 0.78)),
                ));
            });
        });
}

pub(crate) fn setup_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.09)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Preparando mundo..."),
                TextFont::from_font_size(34.0),
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Text::new("Cargando terreno: 0%"),
                TextFont::from_font_size(20.0),
                TextColor(Color::srgb(0.72, 0.88, 0.78)),
                LoadingText,
            ));
            root.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(46.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.2, 0.25)),
                MenuAction(MenuCommand::CancelLoading),
            ))
            .with_child((
                Text::new("Cancelar"),
                TextFont::from_font_size(18.0),
                TextColor(Color::WHITE),
            ));
        });
}

pub(crate) fn setup_pause_menu(mut commands: Commands, storage: Res<WorldStorage>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.88)),
            PauseRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Juego en pausa"),
                TextFont::from_font_size(38.0),
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Text::new(format!("Autoguardado: {}", storage.status)),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.82, 0.86, 0.9)),
                PauseStatus,
            ));
            for (label, action) in [
                ("Continuar", MenuCommand::Resume),
                ("Guardar y volver al menú", MenuCommand::SaveAndMenu),
            ] {
                root.spawn((
                    Button,
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Px(54.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.2, 0.25)),
                    MenuAction(action),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                ));
            }
        });
}

pub(crate) fn cleanup_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}

pub(crate) fn cleanup_pause_menu(mut commands: Commands, roots: Query<Entity, With<PauseRoot>>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}

pub(crate) fn main_menu_actions(
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Play => next.set(AppState::WorldSelect),
            MenuCommand::Create => {
                commands.insert_resource(WorldForm::default());
                next.set(AppState::CreateWorld);
            }
            MenuCommand::Exit => {
                exit.send(AppExit::Success);
            }
            _ => {}
        }
    }
}

pub(crate) fn world_select_actions(
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    select_buttons: Query<(&Interaction, &SelectWorldAction), Changed<Interaction>>,
    mut world_rows: Query<(&SelectWorldAction, &mut BackgroundColor)>,
    mut selection_status: Query<&mut Text, With<WorldSelectionStatus>>,
    mut selected: ResMut<SelectedWorld>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Back => next.set(AppState::MainMenu),
            MenuCommand::Create => {
                commands.insert_resource(WorldForm::default());
                next.set(AppState::CreateWorld);
            }
            MenuCommand::PlaySelected => {
                let Some(path) = selected.path.as_ref() else {
                    continue;
                };
                match load_world(path) {
                    Ok(save) => {
                        pending.save = Some(save);
                        pending.path = Some(path.clone());
                        pending.is_new_world = false;
                        *storage = WorldStorage::default();
                        next.set(AppState::LoadingWorld);
                    }
                    Err(error) => error!("Could not load world {:?}: {error}", path),
                }
            }
            MenuCommand::DeleteSelected if selected.path.is_some() => {
                next.set(AppState::ConfirmDeleteWorld);
            }
            MenuCommand::ConfirmDelete => {
                if let Some(path) = selected.path.as_ref() {
                    let result = fs::remove_file(path)
                        .map_err(|error| format!("No se pudo eliminar el mundo: {error}"))
                        .and_then(|()| {
                            let chunks = chunk_directory(path);
                            if chunks.exists() {
                                fs::remove_dir_all(&chunks).map_err(|error| {
                                    format!("No se pudieron eliminar los chunks: {error}")
                                })?;
                            }
                            Ok(())
                        });
                    match result {
                        Ok(()) => {
                            *selected = SelectedWorld::default();
                            next.set(AppState::WorldSelect);
                        }
                        Err(error) => {
                            error!("{error}");
                            next.set(AppState::WorldSelect);
                        }
                    }
                } else {
                    next.set(AppState::WorldSelect);
                }
            }
            MenuCommand::CancelDelete => next.set(AppState::WorldSelect),
            _ => {}
        }
    }
    for (interaction, action) in &select_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        selected.path = Some(action.path.clone());
        selected.name = Some(action.name.clone());
        for (row, mut background) in &mut world_rows {
            background.0 = if row.path == action.path {
                Color::srgb(0.18, 0.36, 0.29)
            } else {
                Color::srgb(0.12, 0.2, 0.25)
            };
        }
        for mut text in &mut selection_status {
            **text = format!("Seleccionado: {}", action.name).into();
        }
    }
}

pub(crate) fn create_world_input(
    mut events: EventReader<KeyboardInput>,
    mut form: ResMut<WorldForm>,
) {
    let mut changed = false;
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        changed = true;
        match &event.logical_key {
            Key::Enter | Key::Tab => {
                form.focus = if form.focus == WorldField::Name {
                    WorldField::Seed
                } else {
                    WorldField::Name
                };
            }
            Key::Backspace => match form.focus {
                WorldField::Name => {
                    form.name.pop();
                }
                WorldField::Seed => {
                    form.seed.pop();
                }
            },
            Key::Character(character) => match form.focus {
                WorldField::Name if form.name.chars().count() < 32 => {
                    if !character.chars().any(char::is_control) {
                        form.name.push_str(character);
                    }
                }
                WorldField::Seed
                    if form.seed.len() < 20
                        && character
                            .chars()
                            .all(|character| character.is_ascii_digit()) =>
                {
                    form.seed.push_str(character);
                }
                _ => {}
            },
            _ => {}
        }
    }
    if changed {
        form.error = None;
    }
}

pub(crate) fn create_world_actions(
    mut form: ResMut<WorldForm>,
    mut next: ResMut<NextState<AppState>>,
    mut pending: ResMut<PendingWorld>,
    mut storage: ResMut<WorldStorage>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    fields: Query<(&Interaction, &FormField), Changed<Interaction>>,
) {
    for (interaction, field) in &fields {
        if *interaction == Interaction::Pressed {
            form.focus = field.0;
            form.error = None;
        }
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Back => next.set(AppState::MainMenu),
            MenuCommand::StartWorld => {
                let name = form.name.trim();
                if name.is_empty() {
                    form.error = Some("Escribe un nombre para el mundo.".into());
                    continue;
                }
                let seed = match form.seed.parse::<u64>() {
                    Ok(seed) => seed,
                    Err(_) => {
                        form.error = Some("La semilla debe ser un número válido.".into());
                        continue;
                    }
                };
                let path = match world_file_path(name) {
                    Ok(path) => path,
                    Err(error) => {
                        form.error = Some(error);
                        continue;
                    }
                };
                if path.exists() {
                    form.error = Some("Ya existe un mundo con ese nombre.".into());
                    continue;
                }
                pending.save = Some(WorldSave {
                    version: SAVE_VERSION,
                    name: name.into(),
                    seed,
                    terrain_version: TERRAIN_VERSION,
                    time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
                    player_position: [8.5, 64.0, 8.5],
                    player_yaw: 0.0,
                    player_pitch: -0.3,
                    selected_block: GRASS,
                    selected_hotbar_slot: 0,
                    inventory: [0; LEGACY_BLOCK_COUNT],
                    extra_inventory: vec![0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
                    slot_inventory: Some(Inventory::default()),
                    health: MAX_HEALTH,
                    hunger: MAX_HUNGER,
                    experience: 0,
                    food: default_starting_food(),
                    pickaxe_durability: 0,
                    axe_durability: 0,
                    shovel_durability: 0,
                    sword_durability: 0,
                    chunks: Vec::new(),
                    furnaces: Vec::new(),
                });
                pending.path = Some(path);
                pending.is_new_world = true;
                *storage = WorldStorage::default();
                next.set(AppState::LoadingWorld);
            }
            _ => {}
        }
    }
}

pub(crate) fn update_world_form_ui(
    form: Res<WorldForm>,
    mut name_text: Query<
        &mut Text,
        (
            With<NameInputText>,
            Without<SeedInputText>,
            Without<FormErrorText>,
        ),
    >,
    mut seed_text: Query<
        &mut Text,
        (
            With<SeedInputText>,
            Without<NameInputText>,
            Without<FormErrorText>,
        ),
    >,
    mut error_text: Query<
        &mut Text,
        (
            With<FormErrorText>,
            Without<NameInputText>,
            Without<SeedInputText>,
        ),
    >,
) {
    if !form.is_changed() {
        return;
    }
    if let Ok(mut text) = name_text.get_single_mut() {
        text.0 = format!(
            "{}{}",
            form.name,
            if form.focus == WorldField::Name {
                "_"
            } else {
                ""
            }
        );
    }
    if let Ok(mut text) = seed_text.get_single_mut() {
        text.0 = format!(
            "{}{}",
            form.seed,
            if form.focus == WorldField::Seed {
                "_"
            } else {
                ""
            }
        );
    }
    if let Ok(mut text) = error_text.get_single_mut() {
        text.0 = form.error.clone().unwrap_or_default();
    }
}

pub(crate) fn loading_world_actions(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut pending: ResMut<PendingWorld>,
    mut progress: ResMut<LoadingProgress>,
) {
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed && matches!(action.0, MenuCommand::CancelLoading) {
            progress.initialized = false;
            pending.save = None;
            pending.is_new_world = false;
            next.set(AppState::MainMenu);
        }
    }
}

pub(crate) fn pause_menu_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    world: Res<WorldData>,
    player: Query<(&Transform, &Player)>,
    inventory: Res<Inventory>,
    survival: Res<Survival>,
    crafted_tools: Res<CraftedTools>,
    selected: Res<SelectedBlock>,
    selected_slot: Res<SelectedHotbarSlot>,
    day_night: Res<DayNightCycle>,
    mut storage: ResMut<WorldStorage>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut status: Query<&mut Text, With<PauseStatus>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::InGame);
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action.0 {
            MenuCommand::Resume => next.set(AppState::InGame),
            MenuCommand::SaveAndMenu => {
                let Ok((transform, player)) = player.get_single() else {
                    error!("Cannot save: player entity is missing");
                    set_pause_error(&mut status, "No se pudo encontrar al jugador.");
                    continue;
                };
                let Some(path) = storage.path.clone() else {
                    set_pause_error(&mut status, "No se encontró la ruta del mundo.");
                    continue;
                };
                let save = snapshot_world(
                    &world,
                    transform.translation,
                    player,
                    &inventory,
                    selected.0,
                    selected_slot.0,
                    day_night.time_of_day_seconds,
                    &survival,
                    &crafted_tools,
                );
                match save_all_chunks(&world, &mut storage, &path)
                    .and_then(|()| save_world(&path, save))
                {
                    Ok(()) => {
                        storage.metadata_save_requested = false;
                        storage.status = "Guardado".into();
                        next.set(AppState::MainMenu);
                    }
                    Err(error) => {
                        error!("Could not save world: {error}");
                        storage.status = format!("Error al guardar: {error}");
                        set_pause_error(&mut status, &format!("Error al guardar: {error}"));
                    }
                }
            }
            _ => {}
        }
    }
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

pub(crate) fn set_pause_error(status: &mut Query<&mut Text, With<PauseStatus>>, message: &str) {
    if let Ok(mut text) = status.get_single_mut() {
        text.0 = message.to_string();
    }
}

pub(crate) fn pause_from_game(
    keys: Res<ButtonInput<KeyCode>>,
    crafting_ui: Res<CraftingUi>,
    mut next: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if crafting_ui.is_active() || crafting_ui.just_closed {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
    next.set(AppState::Paused);
}
