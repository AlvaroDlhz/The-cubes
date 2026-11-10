use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::state::app::StatesPlugin;

fn temporary_world_path() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("the-cubes-test-{}-{nonce}", std::process::id()))
        .join("world.ron")
}

fn mining_test_app(block: u8, tools: CraftedTools) -> (App, Entity, Entity) {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(0, 1, 2), block);
    world.chunks.insert(IVec3::ZERO, chunk);
    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let visuals = MiningVisuals {
        materials: create_mining_materials(&mut images, &mut materials),
    };
    let overlay_material = visuals.materials[0].clone();
    let mut inventory = Inventory::default();
    for (kind, durability) in [
        (ToolKind::Pickaxe, tools.pickaxe_durability),
        (ToolKind::Axe, tools.axe_durability),
        (ToolKind::Shovel, tools.shovel_durability),
        (ToolKind::Sword, tools.sword_durability),
    ] {
        if durability > 0 {
            inventory
                .try_add(
                    Item::Tool {
                        kind,
                        durability,
                        material: ToolMaterial::Stone,
                    },
                    1,
                )
                .unwrap();
        }
    }
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .insert_resource(PointerState::default())
        .insert_resource(CraftingUi::default())
        .insert_resource(ChatState::default())
        .insert_resource(SelectedBlock::default())
        .insert_resource(inventory)
        .insert_resource(SelectedHotbarSlot::default())
        .insert_resource(Survival::default())
        .insert_resource(tools)
        .insert_resource(MiningState::default())
        .insert_resource(visuals)
        .insert_resource(world)
        .insert_resource(WorldStorage::default())
        .insert_resource(images)
        .insert_resource(materials)
        .init_resource::<Assets<Mesh>>()
        .add_systems(Update, (edit_blocks, update_mining_visual).chain());
    let mut window = Window::default();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    app.world_mut().spawn((window, PrimaryWindow));
    let player = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.5, 1.5, 4.5),
            Player {
                yaw: 0.0,
                pitch: 0.0,
                velocity: Vec3::ZERO,
                grounded: false,
                flying: false,
                jump_buffer: 0.0,
                coyote_time: 0.0,
                step_up_remaining: 0.0,
                jump_key_down: false,
                flight_key_down: false,
            },
        ))
        .id();
    let overlay = app
        .world_mut()
        .spawn((
            Transform::default(),
            Visibility::Hidden,
            MeshMaterial3d(overlay_material),
            MiningOverlay,
        ))
        .id();
    (app, player, overlay)
}

fn dropped_items(app: &mut App, item: Item) -> usize {
    let world = app.world_mut();
    world
        .query::<&BlockDrop>()
        .iter(world)
        .filter(|drop| drop.item == item)
        .count()
}

fn mining_tick(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(seconds));
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}

#[test]
fn inventory_ui_moves_crafts_and_equips_items() {
    let mut app = App::new();
    let mut inventory = Inventory::default();
    inventory.try_add(Item::Block(WOOD), 1).unwrap();
    app.insert_resource(inventory)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<CraftingUi>()
        .init_resource::<ChatState>()
        .init_resource::<WorldData>()
        .init_resource::<SelectedBlock>()
        .init_resource::<SelectedHotbarSlot>()
        .add_systems(
            Update,
            (
                inventory_controls,
                inventory_clicks,
                sync_selected_block,
                update_inventory_ui,
            )
                .chain(),
        );
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.world_mut().spawn((Node::default(), InventoryPanel));
    let slot0 = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::Slot(0)))
        .id();
    let grid0 = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::Small(0)))
        .id();
    let result = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::SmallResult))
        .id();
    let slot1 = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::Slot(1)))
        .id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyE);
    app.update();
    assert!(app.world().resource::<CraftingUi>().inventory_open);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyE);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    for entity in [slot0, grid0, result, slot1] {
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Hovered;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
    }
    let inventory = app.world().resource::<Inventory>();
    assert_eq!(
        inventory.slots[1],
        Some(ItemStack {
            item: Item::Block(PLANKS),
            count: 4
        })
    );
    assert!(inventory.cursor.is_none());
    assert!(inventory.small_grid.iter().all(Option::is_none));
    app.world_mut().resource_mut::<SelectedHotbarSlot>().0 = 1;
    app.update();
    assert_eq!(app.world().resource::<SelectedBlock>().0, PLANKS);
}

#[test]
fn world_save_restores_slots_cursor_grids_and_selected_tool() {
    let mut inventory = Inventory::default();
    inventory.slots[3] = Some(ItemStack {
        item: Item::Tool {
            kind: ToolKind::Sword,
            durability: 15,
            material: ToolMaterial::Stone,
        },
        count: 1,
    });
    inventory.cursor = Some(ItemStack {
        item: Item::Block(STONE),
        count: 3,
    });
    inventory.small_grid[1] = Some(ItemStack {
        item: Item::Block(WOOD),
        count: 4,
    });
    inventory.large_grid[8] = Some(ItemStack {
        item: Item::Arrow,
        count: 5,
    });
    let player = Player {
        yaw: 0.0,
        pitch: 0.0,
        velocity: Vec3::ZERO,
        grounded: false,
        flying: false,
        jump_buffer: 0.0,
        coyote_time: 0.0,
        step_up_remaining: 0.0,
        jump_key_down: false,
        flight_key_down: false,
    };
    let world = WorldData {
        name: "Prueba".into(),
        ..default()
    };
    let save = snapshot_world(
        &world,
        Vec3::ZERO,
        &player,
        &inventory,
        AIR,
        3,
        DEFAULT_TIME_OF_DAY_SECONDS,
        &Survival::default(),
        &CraftedTools::default(),
    );
    let encoded = ron::to_string(&save).unwrap();
    let loaded: WorldSave = ron::from_str(&encoded).unwrap();
    validate_world_save(&loaded).unwrap();
    let mut app = App::new();
    app.add_plugins(StatesPlugin)
        .init_state::<AppState>()
        .init_resource::<LoadingProgress>()
        .init_resource::<WorldData>()
        .init_resource::<DayNightCycle>()
        .init_resource::<Inventory>()
        .init_resource::<Survival>()
        .init_resource::<CraftedTools>()
        .init_resource::<CraftingUi>()
        .init_resource::<SelectedBlock>()
        .init_resource::<SelectedHotbarSlot>()
        .init_resource::<WorldStorage>()
        .init_resource::<ChunkStreamingState>()
        .insert_resource(PendingWorld {
            save: Some(loaded),
            ..default()
        })
        .add_systems(Update, begin_loading_world);
    app.update();
    let restored = app.world().resource::<Inventory>();
    assert_eq!(restored.slots, inventory.slots);
    assert_eq!(restored.cursor, inventory.cursor);
    assert_eq!(restored.small_grid, inventory.small_grid);
    assert_eq!(restored.large_grid, inventory.large_grid);
    assert_eq!(app.world().resource::<SelectedHotbarSlot>().0, 3);
    assert_eq!(app.world().resource::<SelectedBlock>().0, AIR);
}

#[test]
fn mining_requires_hold_and_shows_progress_until_the_block_breaks() {
    let (mut app, _, overlay) = mining_test_app(STONE, CraftedTools::default());
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 2.9);
    let position = IVec3::new(0, 1, 2);
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), position),
        STONE
    );
    assert_eq!(app.world().resource::<Inventory>().count(STONE), 0);
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        app.world().get::<Transform>(overlay).unwrap().translation,
        position.as_vec3() + Vec3::splat(0.5)
    );
    assert_eq!(
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(overlay)
            .unwrap()
            .0,
        app.world().resource::<MiningVisuals>().materials[7]
    );
    assert_eq!(dropped_items(&mut app, Item::Block(STONE)), 0);
    mining_tick(&mut app, 0.11);
    assert_eq!(block_at(app.world().resource::<WorldData>(), position), AIR);
    // Broken blocks become world drops; they reach the inventory on pickup.
    assert_eq!(app.world().resource::<Inventory>().count(STONE), 0);
    assert_eq!(dropped_items(&mut app, Item::Block(STONE)), 1);
    assert_eq!(app.world().resource::<Survival>().experience, 2);
    assert!(app
        .world()
        .resource::<WorldStorage>()
        .dirty_chunks
        .contains(&IVec3::ZERO));
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn mining_tools_halve_time_and_only_wear_after_a_successful_break() {
    let tools = CraftedTools {
        pickaxe_durability: 1,
        ..default()
    };
    let (mut app, _, _) = mining_test_app(STONE, tools);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 1.49);
    assert!(matches!(
        app.world().resource::<Inventory>().slots[0],
        Some(ItemStack {
            item: Item::Tool {
                kind: ToolKind::Pickaxe,
                durability: 1,
                material: ToolMaterial::Stone,
            },
            ..
        })
    ));
    assert_eq!(app.world().resource::<Inventory>().count(STONE), 0);
    mining_tick(&mut app, 0.02);
    assert!(app.world().resource::<Inventory>().slots[0].is_none());
    assert_eq!(dropped_items(&mut app, Item::Block(STONE)), 2);
    assert_eq!(app.world().resource::<Survival>().experience, 4);
    assert_eq!(mining_duration(DIRT, Some(MiningTool::Shovel)), 0.4);
    assert_eq!(mining_duration(WOOD, Some(MiningTool::Axe)), 1.0);
    assert_eq!(mining_duration(STONE, None), 3.0);
    assert_eq!(
        selected_mining_tool(STONE, app.world().resource::<Inventory>().slots[0]),
        None
    );
}

#[test]
fn mining_resets_on_release_target_change_and_open_inventory() {
    let (mut app, player, overlay) = mining_test_app(STONE, CraftedTools::default());
    app.world_mut()
        .resource_mut::<WorldData>()
        .chunks
        .get_mut(&IVec3::ZERO)
        .unwrap()
        .set(IVec3::new(1, 1, 2), STONE);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 1.0);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .x = 1.5;
    mining_tick(&mut app, 0.25);
    let target = app.world().resource::<MiningState>().target.unwrap();
    assert_eq!(target.position, IVec3::new(1, 1, 2));
    assert!((target.elapsed - 0.25).abs() < 0.001);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    mining_tick(&mut app, 0.1);
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Hidden
    );
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 0.25);
    app.world_mut().resource_mut::<CraftingUi>().inventory_open = true;
    mining_tick(&mut app, 0.25);
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), IVec3::new(1, 1, 2)),
        STONE
    );
}

#[test]
fn holding_attack_does_not_repeat_mob_hits_or_mine_through_mobs() {
    let tools = CraftedTools {
        sword_durability: 3,
        ..default()
    };
    let (mut app, _, _) = mining_test_app(STONE, tools);
    let mob = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.5, 0.7, 3.0),
            Mob {
                kind: MobKind::Grazer,
                health: 6,
                attack_elapsed: 0.0,
                wander_elapsed: 0.0,
                wander_angle: 0.0,
                wander_step: 0,
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 0.25);
    mining_tick(&mut app, 3.0);
    assert_eq!(app.world().get::<Mob>(mob).unwrap().health, 4);
    assert!(matches!(
        app.world().resource::<Inventory>().slots[0],
        Some(ItemStack {
            item: Item::Tool {
                kind: ToolKind::Sword,
                durability: 2,
                material: ToolMaterial::Stone,
            },
            ..
        })
    ));
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 2)),
        STONE
    );
}

#[test]
fn right_click_cancels_mining_and_still_places_blocks() {
    let (mut app, _, overlay) = mining_test_app(STONE, CraftedTools::default());
    app.world_mut()
        .resource_mut::<Inventory>()
        .try_add(Item::Block(GRASS), 1)
        .unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 1.0);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    mining_tick(&mut app, 0.1);
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 2)),
        STONE
    );
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), IVec3::new(0, 1, 3)),
        GRASS
    );
    assert_eq!(app.world().resource::<Inventory>().count(GRASS), 0);
}

#[test]
fn pausing_clears_mining_progress_and_hides_overlay() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin)
        .init_state::<AppState>()
        .init_resource::<MiningState>()
        .add_systems(OnExit(AppState::InGame), reset_mining);
    let overlay = app
        .world_mut()
        .spawn((Visibility::Visible, MiningOverlay))
        .id();
    let outline = app
        .world_mut()
        .spawn((Visibility::Visible, BlockOutline))
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    app.world_mut().resource_mut::<MiningState>().target = Some(MiningTarget {
        position: IVec3::ZERO,
        block: STONE,
        tool: None,
        material: ToolMaterial::Stone,
        elapsed: 1.0,
        duration: 3.0,
    });
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Paused);
    app.update();
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert_eq!(
        *app.world().get::<Visibility>(overlay).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        *app.world().get::<Visibility>(outline).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn mining_cracks_increase_with_each_stage() {
    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let stages = create_mining_materials(&mut images, &mut materials);
    assert_eq!(stages.len(), MINING_CRACK_STAGES);
    let mut previous_count = 0;
    for handle in stages {
        let material = materials.get(&handle).unwrap();
        let image = images
            .get(material.base_color_texture.as_ref().unwrap())
            .unwrap();
        assert_eq!(image.texture_descriptor.size.width, 32);
        assert_eq!(image.texture_descriptor.size.height, 32);
        let count = image
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] != 0)
            .count();
        assert!(count > previous_count);
        assert!(count < 32 * 32);
        previous_count = count;
    }
}

#[test]
fn loading_creates_player_and_textured_chunks_before_saving() {
    let path = temporary_world_path();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_state::<AppState>()
        .init_resource::<TextureAssets>()
        .init_resource::<MobAssets>()
        .init_resource::<PendingWorld>()
        .init_resource::<LoadingProgress>()
        .init_resource::<WorldData>()
        .init_resource::<Inventory>()
        .init_resource::<Survival>()
        .init_resource::<CraftedTools>()
        .init_resource::<MiningState>()
        .init_resource::<MiningVisuals>()
        .init_resource::<SelectedBlock>()
        .init_resource::<SelectedHotbarSlot>()
        .init_resource::<DayNightCycle>()
        .init_resource::<ChunkStreamingState>()
        .insert_resource(WorldStorage {
            path: Some(path.clone()),
            ..default()
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(WorldScenePlugin)
        .add_systems(
            Update,
            loading_world_step.run_if(in_state(AppState::LoadingWorld)),
        )
        .add_systems(
            Update,
            pause_menu_actions.run_if(in_state(AppState::Paused)),
        );

    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::LoadingWorld);
    app.update();
    let atlas = app
        .world()
        .resource::<TextureAssets>()
        .atlas
        .clone()
        .expect("atlas created");
    assert!(app
        .world()
        .resource::<Assets<Image>>()
        .get(&atlas)
        .is_some());

    {
        let mut world = app.world_mut().resource_mut::<WorldData>();
        world.name = "Prueba".into();
        world.chunks.insert(IVec3::ZERO, Chunk::new());
    }
    {
        let mut progress = app.world_mut().resource_mut::<LoadingProgress>();
        progress.initialized = true;
        progress.stage = LoadingStage::Meshes;
        progress.coordinates.push(IVec3::ZERO);
        progress.total = 1;
    }
    app.update();
    let material = app
        .world()
        .resource::<LoadingProgress>()
        .material
        .clone()
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(&material)
            .unwrap()
            .base_color_texture,
        Some(atlas)
    );
    app.update();
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world());
    let position = app.world().get::<Transform>(player).unwrap().translation;

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Paused);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Player>>()
            .single(app.world()),
        player
    );

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Paused);
    app.update();
    app.world_mut()
        .spawn((Interaction::Pressed, MenuAction(MenuCommand::SaveAndMenu)));
    app.update();
    let save = load_world(&path).expect("player and world saved");
    assert_eq!(save.player_position, position.to_array());
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    fs::remove_file(&path).expect("remove temporary world file");
    fs::remove_dir(path.parent().unwrap()).expect("remove temporary world directory");
}

#[test]
fn generated_atlas_contains_each_block_texture_and_grass_top() {
    use bevy::image::{CompressedImageFormats, ImageType};

    let atlas = include_bytes!(concat!(env!("OUT_DIR"), "/block_atlas.rgba"));
    let size = texture_layout::ATLAS_SIZE as usize;
    let tile_size = texture_layout::TILE_SIZE as usize;
    assert_eq!(atlas.len(), size * size * 4);
    let mut tiles = HashSet::new();
    for &(tile, filename) in texture_layout::TEXTURE_FILES {
        assert!(tiles.insert(tile), "duplicate tile {tile}");
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/blocks")
            .join(filename);
        let bytes = fs::read(&path).expect("read source texture");
        let image = Image::from_buffer(
            &bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::nearest(),
            RenderAssetUsages::MAIN_WORLD,
        )
        .expect("decode source texture");
        assert_eq!(image.texture_descriptor.size.width as usize, tile_size);
        assert_eq!(image.texture_descriptor.size.height as usize, tile_size);
        let cell = texture_layout::CELL_SIZE as usize;
        let gutter = texture_layout::TILE_GUTTER as usize;
        let x = tile % texture_layout::ATLAS_COLUMNS as usize * cell;
        let y = tile / texture_layout::ATLAS_COLUMNS as usize * cell;
        for row in 0..tile_size {
            let offset = ((y + gutter + row) * size + x + gutter) * 4;
            let source = row * tile_size * 4;
            assert_eq!(
                &atlas[offset..offset + tile_size * 4],
                &image.data[source..source + tile_size * 4],
                "{filename}, row {row}"
            );
        }
        // The border around the tile repeats its nearest edge pixel, so a
        // sample just outside the tile never reads a neighbouring texture.
        for cell_y in 0..cell {
            for cell_x in 0..cell {
                let source_x = cell_x.saturating_sub(gutter).min(tile_size - 1);
                let source_y = cell_y.saturating_sub(gutter).min(tile_size - 1);
                let offset = ((y + cell_y) * size + x + cell_x) * 4;
                let source = (source_y * tile_size + source_x) * 4;
                assert_eq!(
                    &atlas[offset..offset + 4],
                    &image.data[source..source + 4],
                    "{filename}, cell pixel ({cell_x}, {cell_y})"
                );
            }
        }
        assert_eq!(
            texture_layout::TEXTURE_OPAQUE[tile],
            image.data.chunks_exact(4).all(|pixel| pixel[3] == 255),
            "{filename}"
        );
    }
    for tile in 1..BLOCK_COUNT {
        assert!(tiles.contains(&tile), "missing block tile {tile}");
    }
    let (grass_top, _, _, _) = rendering::block_uv(GRASS, 2, 0);
    let (grass_side, _, _, _) = rendering::block_uv(GRASS, 0, 0);
    let top_tile = texture_layout::texture_index("grass_top");
    let side_tile = texture_layout::texture_index("grass_side");
    let (top_min, _) = texture_layout::tile_uv_rect(top_tile);
    let (side_min, side_max) = texture_layout::tile_uv_rect(side_tile);
    assert_eq!(grass_top, top_min);
    assert_eq!(grass_side, [side_min[0], side_max[1]]);
    // UVs cover exactly the tile, leaving the repeated border outside.
    let atlas_size = texture_layout::ATLAS_SIZE as f32;
    assert_eq!(
        (side_max[0] - side_min[0]) * atlas_size,
        texture_layout::TILE_SIZE as f32
    );
    assert_eq!(
        side_min[0] * atlas_size,
        (side_tile as u32 % texture_layout::ATLAS_COLUMNS * texture_layout::CELL_SIZE
            + texture_layout::TILE_GUTTER) as f32
    );
}

#[test]
fn block_address_handles_negative_world_coordinates() {
    let (chunk, local) = block_address(IVec3::new(-1, -16, -17));
    assert_eq!(chunk, IVec3::new(-1, -1, -2));
    assert_eq!(local, IVec3::new(15, 0, 15));
}

#[test]
fn block_index_covers_chunk_without_collisions() {
    let indices = (0..CHUNK_SIZE)
        .flat_map(|y| {
            (0..CHUNK_SIZE)
                .flat_map(move |z| (0..CHUNK_SIZE).map(move |x| block_index(IVec3::new(x, y, z))))
        })
        .collect::<HashSet<_>>();
    assert_eq!(indices.len(), CHUNK_VOLUME);
    assert_eq!(indices.iter().min(), Some(&0));
    assert_eq!(indices.iter().max(), Some(&(CHUNK_VOLUME - 1)));
}

#[test]
fn terrain_height_is_deterministic_and_inside_chunk() {
    assert_eq!(terrain_height(13, -9, 123), terrain_height(13, -9, 123));
    assert_ne!(terrain_height(13, -9, 123), terrain_height(13, -9, 999_999));
    for x in -128..128 {
        for z in -128..128 {
            let height = terrain_height(x, z, 42);
            assert!((8..=WORLD_HEIGHT - 16).contains(&height), "height {height}");
        }
    }
}

#[test]
fn new_worlds_generate_connected_vertical_terrain_and_preserve_legacy_worlds() {
    let seed = 42;
    let mut found_cave = false;
    let mut found_water = false;
    // Seed 42 is low-lying near the origin, so search far enough to reach
    // columns tall enough to hold a cave above sea level.
    'search: for z in -128..128 {
        for x in -128..128 {
            let height = terrain_height(x, z, seed);
            if !found_cave && height > SEA_LEVEL + 8 {
                for y in SEA_LEVEL + 1..height - 4 {
                    if !is_cave(x, y, z, seed) {
                        continue;
                    }
                    let coord = IVec3::new(
                        x.div_euclid(CHUNK_SIZE),
                        y.div_euclid(CHUNK_SIZE),
                        z.div_euclid(CHUNK_SIZE),
                    );
                    let local = IVec3::new(
                        x.rem_euclid(CHUNK_SIZE),
                        y.rem_euclid(CHUNK_SIZE),
                        z.rem_euclid(CHUNK_SIZE),
                    );
                    assert_eq!(generate_chunk(coord, seed).get(local), AIR);
                    found_cave = true;
                    break;
                }
            }

            if !found_water && height < SEA_LEVEL - 1 {
                let coord = IVec3::new(
                    x.div_euclid(CHUNK_SIZE),
                    SEA_LEVEL.div_euclid(CHUNK_SIZE),
                    z.div_euclid(CHUNK_SIZE),
                );
                let local = IVec3::new(
                    x.rem_euclid(CHUNK_SIZE),
                    SEA_LEVEL.rem_euclid(CHUNK_SIZE),
                    z.rem_euclid(CHUNK_SIZE),
                );
                assert_eq!(generate_chunk(coord, seed).get(local), WATER);
                found_water = true;
            }
            if found_cave && found_water {
                break 'search;
            }
        }
    }
    assert!(found_cave, "seeded terrain should contain a dry cave");
    assert!(found_water, "low terrain should fill to sea level");

    let legacy = WorldData {
        seed,
        terrain_version: 1,
        ..default()
    };
    let legacy_surface = chunk_for_coordinate(&legacy, IVec3::ZERO);
    assert_eq!(
        legacy_surface.blocks,
        generate_legacy_chunk(IVec3::ZERO, seed).blocks
    );
    assert!(chunk_for_coordinate(&legacy, IVec3::Y)
        .blocks
        .iter()
        .all(|block| *block == AIR));
}

#[test]
fn generated_chunks_include_varied_terrain_and_new_blocks() {
    let mut biomes = HashSet::new();
    let mut present = HashSet::new();
    let mut found_mountain = false;
    let mut found_desert = false;
    let deterministic_chunk = generate_chunk(IVec3::new(-4, 3, -4), 42);
    let mut generated_chunks = HashMap::new();

    for chunk_z in -4..=4 {
        for chunk_x in -4..=4 {
            for chunk_y in 0..WORLD_HEIGHT / CHUNK_SIZE {
                let coord = IVec3::new(chunk_x, chunk_y, chunk_z);
                let chunk = generate_chunk(coord, 42);
                present.extend(chunk.blocks.iter().copied());
                generated_chunks.insert(coord, chunk);
            }
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let world_x = chunk_x * CHUNK_SIZE + x;
                    let world_z = chunk_z * CHUNK_SIZE + z;
                    let biome = biome_at(world_x, world_z, 42);
                    let height = terrain_height(world_x, world_z, 42);
                    let coord = IVec3::new(chunk_x, height.div_euclid(CHUNK_SIZE), chunk_z);
                    let local = IVec3::new(x, height.rem_euclid(CHUNK_SIZE), z);
                    assert_eq!(
                        generated_chunks[&coord].get(local),
                        biome.surface_block(),
                        "surface at ({world_x}, {height}, {world_z})"
                    );
                }
            }
        }
    }

    // Biomes span hundreds of blocks, so sample a wide region coarsely
    // to check variety instead of relying on the few chunks built above.
    for world_z in (-1024..1024).step_by(16) {
        for world_x in (-1024..1024).step_by(16) {
            let biome = biome_at(world_x, world_z, 42);
            biomes.insert(biome);
            found_mountain |=
                biome == Biome::Mountain && terrain_height(world_x, world_z, 42) >= 80;
            found_desert |= biome == Biome::Desert;
        }
    }

    assert!(
        biomes.contains(&Biome::Plains),
        "generated biomes: {biomes:?}"
    );
    assert!(biomes.contains(&Biome::Forest));
    assert!(
        biomes.contains(&Biome::Desert),
        "generated biomes: {biomes:?}"
    );
    assert!(biomes.contains(&Biome::Mountain));
    assert!(found_mountain);
    assert!(found_desert);
    assert!(present.contains(&GRASS));
    assert!(present.contains(&DIRT));
    assert!(present.contains(&STONE));
    assert!(present.contains(&SAND));
    assert!(present.contains(&WOOD));
    assert!(present.contains(&LEAVES));
    assert!(present.contains(&WATER));
    assert!(present.iter().all(|block| *block < BLOCK_COUNT as u8));

    // Test that new block types are properly defined
    assert_ne!(ICE, AIR);
    assert_ne!(SNOW, AIR);
    assert_ne!(MUD, AIR);
    assert_ne!(OBSIDIAN, AIR);
    assert_ne!(GRAVEL, AIR);

    assert_eq!(
        generate_chunk(IVec3::new(-4, 3, -4), 42).blocks,
        deterministic_chunk.blocks
    );
}

#[test]
fn streaming_coordinates_cover_the_radius_around_negative_centers() {
    let center = IVec3::new(-2, 0, 3);
    // Fixed radii: the assertions below must not depend on tuning constants.
    let coordinates = chunk_coordinates_around(center, 3, 1);

    assert_eq!(coordinates.len(), 98);
    assert!(coordinates.contains(&center));
    assert!(coordinates.contains(&IVec3::new(-5, 0, 0)));
    assert!(coordinates.contains(&IVec3::new(1, 0, 6)));
    assert!(coordinates
        .iter()
        .all(|coordinate| (0..=1).contains(&coordinate.y)));
}

#[test]
fn unloaded_chunks_regenerate_terrain_and_restore_saved_edits() {
    let mut world = WorldData {
        seed: 812,
        ..default()
    };
    let coord = IVec3::new(-2, 0, 1);
    let generated = chunk_for_coordinate(&world, coord);
    assert_eq!(generated.blocks, generate_chunk(coord, 812).blocks);

    let mut edited = generated;
    edited.set(IVec3::new(0, 0, 0), STONE);
    world.modified_chunks.insert(coord, edited);
    let restored = chunk_for_coordinate(&world, coord);

    assert_eq!(restored.get(IVec3::new(0, 0, 0)), STONE);
    assert_eq!(
        chunk_for_coordinate(&world, IVec3::new(20, 0, 20)).blocks,
        generate_chunk(IVec3::new(20, 0, 20), 812).blocks
    );
}

#[test]
fn adjacent_blocks_keep_separate_faces_and_texture_variants() {
    let mut chunk = Chunk::new();
    chunk.set(IVec3::ZERO, STONE);
    chunk.set(IVec3::X, STONE);
    let world = WorldData {
        chunks: HashMap::from([(IVec3::ZERO, chunk)]),
        ..default()
    };
    let mesh = rendering::build_chunk_mesh(&world, IVec3::ZERO);
    let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        panic!("block mesh positions must be 3D floats");
    };
    let bevy::render::mesh::VertexAttributeValues::Float32x2(uvs) =
        mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
    else {
        panic!("block mesh UVs must be 2D floats");
    };

    let mut top_faces = positions
        .chunks_exact(4)
        .zip(uvs.chunks_exact(4))
        .filter_map(|(vertices, face_uvs)| {
            vertices
                .iter()
                .all(|position| position[1] == 1.0)
                .then_some((vertices, face_uvs))
        })
        .collect::<Vec<_>>();
    top_faces.sort_by(|(left, _), (right, _)| {
        left.iter()
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min)
            .total_cmp(
                &right
                    .iter()
                    .map(|position| position[0])
                    .fold(f32::INFINITY, f32::min),
            )
    });

    assert_eq!(top_faces.len(), 2);
    for (x, (positions, uvs)) in top_faces.into_iter().enumerate() {
        let min_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(max_x - min_x, 1.0);
        assert_eq!(min_x, x as f32);

        let variant = rendering::block_variant(x as i32, 0, 0);
        let (uv0, uv1, uv2, uv3) = rendering::block_uv(STONE, 2, variant);
        assert_eq!(uvs, [uv0, uv1, uv2, uv3]);
    }
}

#[test]
fn player_collision_allows_standing_on_a_block() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::ZERO, STONE);
    world.chunks.insert(IVec3::ZERO, chunk);

    assert!(!collides_with_world(&world, Vec3::new(0.5, 2.62, 0.5)));
    assert!(collides_with_world(&world, Vec3::new(0.5, 2.5, 0.5)));
}

#[test]
fn player_collision_stops_at_walls() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(1, 1, 0), STONE);
    world.chunks.insert(IVec3::ZERO, chunk);

    assert!(!collides_with_world(&world, Vec3::new(0.5, 2.62, 0.5)));
    assert!(collides_with_world(&world, Vec3::new(0.75, 2.62, 0.5)));
}

#[test]
fn player_recovery_only_moves_out_of_actual_solid_collisions() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(0, 0, 0), STONE);
    chunk.set(IVec3::new(1, 1, 0), STONE);
    chunk.set(IVec3::new(1, 2, 0), STONE);
    chunk.set(IVec3::new(2, 1, 0), SLAB);
    world.chunks.insert(IVec3::ZERO, chunk);

    let standing = Vec3::new(0.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
    assert_eq!(find_safe_spawn_position(&world, standing), standing);
    let above_ground = standing + Vec3::Y * 0.5;
    assert_eq!(find_safe_spawn_position(&world, above_ground), above_ground);
    let in_slab = Vec3::new(2.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
    assert_eq!(find_safe_spawn_position(&world, in_slab), in_slab);

    let embedded = Vec3::new(0.75, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
    let recovered = find_safe_spawn_position(&world, embedded);
    assert!(!collides_with_world(&world, recovered));
    assert!((recovered.y - (3.0 + PLAYER_EYE_HEIGHT)).abs() < 0.0001);
    assert_eq!(recovered.xz(), embedded.xz());
}

#[test]
fn stationary_player_stays_on_ground_and_can_jump() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::ZERO, STONE);
    world.chunks.insert(IVec3::ZERO, chunk);

    let mut app = App::new();
    let mut time = Time::<Fixed>::default();
    time.advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
    app.insert_resource(time)
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(world)
        .insert_resource(Survival::default())
        .add_systems(Update, move_player);
    let mut window = Window::default();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    app.world_mut().spawn((window, PrimaryWindow));
    let start = Vec3::new(0.5, 1.0 + PLAYER_EYE_HEIGHT, 0.5);
    let entity = app
        .world_mut()
        .spawn((
            Transform::from_translation(start),
            Player {
                yaw: 0.0,
                pitch: 0.0,
                velocity: Vec3::ZERO,
                grounded: true,
                flying: false,
                jump_buffer: 0.0,
                coyote_time: 0.0,
                step_up_remaining: 0.0,
                jump_key_down: false,
                flight_key_down: false,
            },
        ))
        .id();

    for _ in 0..120 {
        app.update();
        let position = app.world().get::<Transform>(entity).unwrap().translation;
        assert!((position.y - start.y).abs() < 0.001, "{position:?}");
        assert!(app.world().get::<Player>(entity).unwrap().grounded);
    }

    app.world_mut()
        .get_mut::<Transform>(entity)
        .unwrap()
        .translation
        .y += 2.0;
    app.world_mut().get_mut::<Player>(entity).unwrap().grounded = false;
    for _ in 0..240 {
        app.update();
    }
    let landed = app.world().get::<Transform>(entity).unwrap().translation;
    assert!((landed.y - start.y).abs() < 0.001, "{landed:?}");
    assert!(app.world().get::<Player>(entity).unwrap().grounded);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    assert!(app.world().get::<Transform>(entity).unwrap().translation.y > start.y);
}

#[test]
fn flying_inside_blocks_is_not_interrupted_by_recovery() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::ZERO, STONE);
    world.chunks.insert(IVec3::ZERO, chunk);

    let mut app = App::new();
    let mut time = Time::<Fixed>::default();
    time.advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
    app.insert_resource(time)
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(world)
        .insert_resource(Survival::default())
        .add_systems(Update, move_player);
    let mut window = Window::default();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    app.world_mut().spawn((window, PrimaryWindow));
    let start = Vec3::new(0.5, 1.0, 0.5);
    let entity = app
        .world_mut()
        .spawn((
            Transform::from_translation(start),
            Player {
                yaw: 0.0,
                pitch: 0.0,
                velocity: Vec3::ZERO,
                grounded: false,
                flying: true,
                jump_buffer: 0.0,
                coyote_time: 0.0,
                step_up_remaining: 0.0,
                jump_key_down: false,
                flight_key_down: false,
            },
        ))
        .id();

    app.update();
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().translation,
        start
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyF);
    app.update();
    let position = app.world().get::<Transform>(entity).unwrap().translation;
    assert!(!collides_with_world(
        app.world().resource::<WorldData>(),
        position
    ));
    assert!((position.y - (1.0 + PLAYER_EYE_HEIGHT)).abs() < 0.001);
}

#[test]
fn inventory_collects_and_consumes_blocks() {
    let mut inventory = Inventory::default();

    assert_eq!(inventory.count(GRASS), 0);
    assert!(!inventory.remove(GRASS));
    assert!(inventory.add(GRASS).is_ok());
    assert_eq!(inventory.count(GRASS), 1);
    assert!(inventory.remove(GRASS));
    assert_eq!(inventory.count(GRASS), 0);
    assert!(inventory.add(AIR).is_err());
}

#[test]
fn hunger_depletes_over_time_and_sprinting_costs_extra() {
    let mut survival = Survival::default();

    advance_survival_stats(&mut survival, PASSIVE_HUNGER_INTERVAL, false);
    assert_eq!(survival.hunger, MAX_HUNGER - 1.0);
    advance_survival_stats(&mut survival, SPRINT_HUNGER_INTERVAL, true);
    assert_eq!(survival.hunger, MAX_HUNGER - 2.0);
}

#[test]
fn eating_restores_hunger_and_consumes_food() {
    let mut survival = Survival {
        hunger: 16.0, // Changed from 15 to 16, so 16 + 4 = 20 = MAX_HUNGER
        food: 2,
        ..default()
    };

    assert!(eat_food(&mut survival));
    assert_eq!(survival.hunger, MAX_HUNGER);
    assert_eq!(survival.food, 1);
    assert!(!eat_food(&mut survival));
    survival.hunger = 0.0;
    assert!(eat_food(&mut survival));
    assert_eq!(survival.hunger, 4.0); // BerryRation provides 4 saturation, not 6
}

#[test]
fn hunger_enables_regeneration_and_starvation_deals_damage() {
    let mut regenerating = Survival {
        health: 18.0,
        ..default()
    };
    advance_survival_stats(&mut regenerating, HEALTH_REGEN_INTERVAL, false);
    assert_eq!(regenerating.health, 19.0);
    assert_eq!(regenerating.hunger, MAX_HUNGER - 0.5);

    let mut starving = Survival {
        health: 3.0,
        hunger: 0.0,
        ..default()
    };
    advance_survival_stats(&mut starving, STARVATION_DAMAGE_INTERVAL * 2.0, false);
    assert_eq!(starving.health, 1.0);
}

#[test]
fn falling_damage_and_experience_levels_have_expected_thresholds() {
    assert_eq!(fall_damage(12.0), 0.0);
    assert_eq!(fall_damage(16.0), 2.0);
    assert_eq!(experience_progress(9), (0, 9, 10));
    assert_eq!(experience_progress(10), (1, 0, 15));
    assert_eq!(experience_progress(26), (2, 1, 20));
    assert_eq!(experience_for_block(STONE), 2);
    assert_eq!(experience_for_block(AIR), 0);
}

#[test]
fn equipped_tools_only_help_with_matching_blocks_and_break() {
    let mut inventory = Inventory::default();
    inventory
        .try_add(
            Item::Tool {
                kind: ToolKind::Pickaxe,
                durability: 1,
                material: ToolMaterial::Stone,
            },
            1,
        )
        .unwrap();
    assert_eq!(
        selected_mining_tool(STONE, inventory.slots[0]),
        Some(MiningTool::Pickaxe)
    );
    assert_eq!(selected_mining_tool(WOOD, inventory.slots[0]), None);
    assert_eq!(wear_selected_tool(&mut inventory, 0), Some(0));
    assert_eq!(selected_mining_tool(STONE, inventory.slots[0]), None);
}

#[test]
fn shovel_is_twice_as_efficient_on_sand_and_dirt() {
    let shovel = Some(ItemStack {
        item: Item::Tool {
            kind: ToolKind::Shovel,
            durability: 10,
            material: ToolMaterial::Stone,
        },
        count: 1,
    });
    assert_eq!(selected_mining_tool(SAND, shovel), Some(MiningTool::Shovel));
    assert_eq!(selected_mining_tool(DIRT, shovel), Some(MiningTool::Shovel));
    assert_eq!(selected_mining_tool(STONE, shovel), None);
    assert_eq!(
        mining_duration(DIRT, selected_mining_tool(DIRT, shovel)),
        mining_duration(DIRT, None) / 2.0
    );
}

#[test]
fn sword_consumes_durability_on_mob_hits() {
    let mut inventory = Inventory::default();
    inventory
        .try_add(
            Item::Tool {
                kind: ToolKind::Sword,
                durability: 5,
                material: ToolMaterial::Stone,
            },
            1,
        )
        .unwrap();
    for remaining in (0..5).rev() {
        assert_eq!(wear_selected_tool(&mut inventory, 0), Some(remaining));
    }
    assert!(inventory.slots[0].is_none());
}

#[test]
fn blocks_cannot_be_placed_inside_the_player() {
    let eye_position = Vec3::new(0.5, 2.62, 0.5);

    assert!(block_overlaps_player(IVec3::new(0, 1, 0), eye_position));
    assert!(!block_overlaps_player(IVec3::new(0, 0, 0), eye_position));
    assert!(!block_overlaps_player(IVec3::new(2, 1, 0), eye_position));
}

#[test]
fn mob_population_caps_each_kind_and_total() {
    assert!(can_spawn_mob(MobKind::Grazer, 4, 3, 1));
    assert!(!can_spawn_mob(MobKind::Grazer, 5, 5, 0));
    assert!(can_spawn_mob(MobKind::NightStalker, 7, 5, 2));
    assert!(!can_spawn_mob(MobKind::NightStalker, 8, 5, 2));
    assert!(!can_spawn_mob(MobKind::NightStalker, 3, 1, 3));
}

#[test]
fn mob_systems_run_without_ecs_query_conflicts() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .insert_resource(DayNightCycle::default())
        .insert_resource(WorldData::default())
        .insert_resource(MobAssets::default())
        .insert_resource(MobPopulation::default())
        .insert_resource(Survival::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .insert_resource(PointerState::default())
        .insert_resource(CraftingUi::default())
        .insert_resource(ChatState::default())
        .insert_resource(SelectedBlock(GRASS))
        .insert_resource(SelectedHotbarSlot::default())
        .insert_resource(Inventory::default())
        .insert_resource(CraftedTools::default())
        .insert_resource(MiningState::default())
        .insert_resource(WorldStorage::default())
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(Update, (update_mobs, edit_blocks).chain());

    app.update();
}

#[test]
fn mobs_spawn_only_at_night_for_hostiles_and_have_defined_rewards() {
    assert!(is_night(0.0));
    assert!(is_night(0.079));
    assert!(!is_night(0.08));
    assert_eq!(mob_rewards(MobKind::Grazer), (2, 3));
    assert_eq!(mob_rewards(MobKind::NightStalker), (0, 5));
}

#[test]
fn mob_ray_targeting_hits_only_entities_in_front_within_reach() {
    let origin = Vec3::ZERO;
    let direction = Vec3::NEG_Z;

    assert_eq!(
        ray_sphere_distance(origin, direction, Vec3::new(0.0, 0.0, -3.0), 0.5),
        Some(2.5)
    );
    assert_eq!(
        ray_sphere_distance(origin, direction, Vec3::new(2.0, 0.0, -3.0), 0.5),
        None
    );
    assert_eq!(
        ray_sphere_distance(origin, direction, Vec3::new(0.0, 0.0, 3.0), 0.5),
        None
    );
}

#[test]
fn block_raycast_reports_the_nearest_surface_for_mob_occlusion() {
    let mut world = WorldData::default();
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(3, 1, 0), STONE);
    world.chunks.insert(IVec3::ZERO, chunk);

    assert_eq!(
        raycast_with_distance(&world, Vec3::new(0.5, 1.5, 0.5), Vec3::X, REACH,),
        Some((IVec3::new(3, 1, 0), IVec3::new(2, 1, 0), 2.5))
    );
}

#[test]
fn mob_spawn_candidates_are_deterministic_and_use_grass_in_loaded_chunks() {
    let seed = 42;
    let mut world = WorldData { seed, ..default() };
    for z in -6..=6 {
        for x in -6..=6 {
            let center_y = terrain_height(
                x * CHUNK_SIZE + CHUNK_SIZE / 2,
                z * CHUNK_SIZE + CHUNK_SIZE / 2,
                seed,
            )
            .div_euclid(CHUNK_SIZE);
            for y in (center_y - 1).max(0)..=(center_y + 1).min(WORLD_HEIGHT / CHUNK_SIZE - 1) {
                let coordinate = IVec3::new(x, y, z);
                world
                    .chunks
                    .insert(coordinate, generate_chunk(coordinate, seed));
            }
        }
    }

    let mut found = None;
    'search: for z in (-64..=64).step_by(8) {
        for x in (-64..=64).step_by(8) {
            let player = Vec3::new(x as f32 + 0.5, 24.0, z as f32 + 0.5);
            for attempt in 0..128 {
                if let Some(position) = mob_spawn_position(&world, player, attempt) {
                    assert_eq!(mob_spawn_position(&world, player, attempt), Some(position));
                    found = Some(position);
                    break 'search;
                }
            }
        }
    }

    let position = found.expect("a loaded plains or forest spawn candidate");
    let x = position.x.floor() as i32;
    let z = position.z.floor() as i32;
    let ground = position.y as i32 - 1;
    assert!(matches!(
        biome_at(x, z, seed),
        Biome::Plains | Biome::Forest
    ));
    assert_eq!(block_at(&world, IVec3::new(x, ground, z)), GRASS);
    assert_eq!(block_at(&world, IVec3::new(x, ground + 1, z)), AIR);
}

#[test]
fn world_save_round_trips_world_and_player_state() {
    let mut world = WorldData {
        name: "Prueba".into(),
        seed: 9876,
        ..default()
    };
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(1, 2, 3), STONE);
    world.chunks.insert(IVec3::ZERO, chunk);
    let mut modified_chunk = Chunk::new();
    modified_chunk.set(IVec3::new(1, 2, 3), STONE);
    world.modified_chunks.insert(IVec3::ZERO, modified_chunk);
    world.chunks.insert(IVec3::new(1, 0, 0), Chunk::new());
    let mut inventory = Inventory::default();
    inventory.try_add(Item::Block(DIRT), 17).unwrap();
    inventory.try_add(Item::Block(SAND), 5).unwrap();
    inventory.try_add(Item::Block(WOOD), 3).unwrap();
    let survival = Survival {
        health: 13.5,
        hunger: 8.0,
        experience: 37,
        food: 2,
        ..default()
    };
    let crafted_tools = CraftedTools {
        pickaxe_durability: 31,
        axe_durability: 12,
        shovel_durability: 0,
        sword_durability: 0,
    };

    let player = Player {
        yaw: 1.2,
        pitch: -0.4,
        velocity: Vec3::ZERO,
        grounded: true,
        flying: false,
        jump_buffer: 0.0,
        coyote_time: 0.0,
        step_up_remaining: 0.0,
        jump_key_down: false,
        flight_key_down: false,
    };
    let save = snapshot_world(
        &world,
        Vec3::new(4.5, 9.0, -2.0),
        &player,
        &inventory,
        DIRT,
        1,
        43_200.0,
        &survival,
        &crafted_tools,
    );
    let contents = ron::to_string(&save).expect("serialize world save");
    let loaded: WorldSave = ron::from_str(&contents).expect("deserialize world save");

    validate_world_save(&loaded).expect("validate world save");
    assert_eq!(loaded.name, "Prueba");
    assert_eq!(loaded.seed, 9876);
    assert_eq!(loaded.time_of_day_seconds, 43_200.0);
    assert_eq!(loaded.player_position, [4.5, 9.0, -2.0]);
    assert_eq!(loaded.player_yaw, 1.2);
    assert_eq!(loaded.player_pitch, -0.4);
    assert_eq!(loaded.selected_block, DIRT);
    assert_eq!(loaded.selected_hotbar_slot, 1);
    assert_eq!(loaded.inventory[DIRT as usize], 17);
    assert_eq!(loaded.extra_inventory[0], 5);
    assert_eq!(loaded.extra_inventory[1], 3);
    assert_eq!(loaded.health, 13.5);
    assert_eq!(loaded.hunger, 8.0);
    assert_eq!(loaded.experience, 37);
    assert_eq!(loaded.food, 2);
    assert_eq!(loaded.pickaxe_durability, 31);
    assert_eq!(loaded.axe_durability, 12);
    assert!(loaded.chunks.is_empty());
}

#[test]
fn per_chunk_storage_and_world_metadata_round_trip() {
    let path = temporary_world_path();
    let coord = IVec3::new(-3, 0, 7);
    let mut chunk = Chunk::new();
    chunk.set(IVec3::new(1, 2, 3), STONE);

    save_chunk_file(&path, coord, &chunk).expect("save chunk");
    let loaded_chunk = load_chunk_file(&path, coord)
        .expect("load chunk file")
        .expect("chunk file exists");
    assert_eq!(loaded_chunk.blocks, chunk.blocks);

    let save = WorldSave {
        version: SAVE_VERSION,
        name: "Prueba".into(),
        seed: 9876,
        terrain_version: TERRAIN_VERSION,
        time_of_day_seconds: 43_200.0,
        player_position: [4.5, 9.0, -2.0],
        player_yaw: 1.2,
        player_pitch: -0.4,
        selected_block: DIRT,
        selected_hotbar_slot: 1,
        inventory: [0, 0, 17, 0],
        extra_inventory: vec![0; BLOCK_COUNT - LEGACY_BLOCK_COUNT],
        slot_inventory: Some(Inventory::default()),
        health: 15.0,
        hunger: 12.0,
        experience: 25,
        food: 4,
        pickaxe_durability: 0,
        axe_durability: 0,
        shovel_durability: 0,
        sword_durability: 0,
        chunks: Vec::new(),
        furnaces: Vec::new(),
    };
    save_world(&path, save).expect("save world metadata");
    let loaded = load_world(&path).expect("load world metadata");
    assert_eq!(loaded.name, "Prueba");
    assert!(loaded.chunks.is_empty());

    fs::remove_dir_all(path.parent().expect("temporary world directory"))
        .expect("remove temporary world files");
}

#[test]
fn save_all_chunks_flushes_active_and_legacy_edits() {
    let path = temporary_world_path();
    let active_coord = IVec3::new(1, 0, 0);
    let legacy_coord = IVec3::new(-1, 0, 0);
    let mut active_chunk = Chunk::new();
    active_chunk.set(IVec3::new(0, 1, 2), STONE);
    let mut legacy_chunk = Chunk::new();
    legacy_chunk.set(IVec3::new(3, 4, 5), DIRT);

    let mut world = WorldData::default();
    world
        .modified_chunks
        .insert(active_coord, active_chunk.clone());
    let mut storage = WorldStorage::default();
    storage
        .legacy_chunks
        .insert(legacy_coord, legacy_chunk.clone());

    save_all_chunks(&world, &mut storage, &path).expect("flush all changed chunks");

    assert!(storage.legacy_chunks.is_empty());
    assert!(storage.dirty_chunks.is_empty());
    assert_eq!(
        load_chunk_file(&path, active_coord)
            .expect("read active chunk")
            .expect("active chunk saved")
            .blocks,
        active_chunk.blocks
    );
    assert_eq!(
        load_chunk_file(&path, legacy_coord)
            .expect("read legacy chunk")
            .expect("legacy chunk saved")
            .blocks,
        legacy_chunk.blocks
    );

    fs::remove_dir_all(path.parent().expect("temporary world directory"))
        .expect("remove temporary world files");
}

#[test]
fn loading_a_legacy_chunk_keeps_it_pending_until_chunk_file_is_written() {
    let coord = IVec3::new(4, 0, -2);
    let mut legacy_chunk = Chunk::new();
    legacy_chunk.set(IVec3::new(2, 3, 4), DIRT);
    let mut storage = WorldStorage::default();
    storage.legacy_chunks.insert(coord, legacy_chunk);
    storage.dirty_chunks.insert(coord);
    let mut world = WorldData::default();

    let loaded =
        load_or_generate_chunk(&mut world, &mut storage, coord).expect("load legacy chunk");

    assert_eq!(loaded.get(IVec3::new(2, 3, 4)), DIRT);
    assert_eq!(world.modified_chunks[&coord].get(IVec3::new(2, 3, 4)), DIRT);
    assert!(storage.dirty_chunks.contains(&coord));
}

#[test]
fn world_save_defaults_new_fields_for_existing_saves() {
    let contents = r#"(version:1,name:"Prueba",seed:42,player_position:(0.0,0.0,0.0),selected_block:1,inventory:(0,0,0,0),chunks:[])"#;
    let save: WorldSave = ron::from_str(contents).expect("deserialize existing world save");

    assert_eq!(save.terrain_version, 1);
    assert_eq!(save.player_yaw, 0.0);
    assert_eq!(save.player_pitch, 0.0);
    assert_eq!(save.time_of_day_seconds, DEFAULT_TIME_OF_DAY_SECONDS);
    assert_eq!(save.selected_hotbar_slot, 0);
    assert_eq!(save.health, MAX_HEALTH);
    assert_eq!(save.hunger, MAX_HUNGER);
    assert_eq!(save.experience, 0);
    assert_eq!(save.food, default_starting_food());
    assert_eq!(save.pickaxe_durability, 0);
    assert_eq!(save.axe_durability, 0);
    assert!(validate_world_save(&save).is_err());
}

#[test]
fn hotbar_has_nine_slots_and_scroll_wraps_in_both_directions() {
    assert_eq!(HOTBAR_SLOT_COUNT, 9);
    assert_eq!(cycle_hotbar_selection(0, -1), 8);
    assert_eq!(cycle_hotbar_selection(8, 1), 0);
    assert_eq!(cycle_hotbar_selection(2, 3), 5);
    assert_eq!(mouse_wheel_steps(1.0, MouseScrollUnit::Line), -1);
    assert_eq!(mouse_wheel_steps(-80.0, MouseScrollUnit::Pixel), 2);
    assert_eq!(mouse_wheel_steps(0.0, MouseScrollUnit::Line), 0);
}

#[test]
fn day_night_cycle_uses_one_real_minute_per_game_hour() {
    let after_one_real_minute = advance_time_of_day(
        DEFAULT_TIME_OF_DAY_SECONDS,
        60.0,
        GAME_SECONDS_PER_REAL_SECOND,
    );
    assert_eq!(after_one_real_minute, 9.0 * 60.0 * 60.0);
    assert_eq!(
        advance_time_of_day(
            after_one_real_minute,
            23.0 * 60.0,
            GAME_SECONDS_PER_REAL_SECOND
        ),
        DEFAULT_TIME_OF_DAY_SECONDS
    );
}

#[test]
fn day_night_cycle_changes_sunlight_and_wraps_clock() {
    let (_, night) = sun_position_and_daylight(0.0);
    let (_, noon) = sun_position_and_daylight(12.0 * 60.0 * 60.0);

    assert!(night < 0.01);
    assert!(noon > 0.99);
    assert_eq!(
        format_time_of_day(23.0 * 60.0 * 60.0 + 59.0 * 60.0),
        "23:59"
    );
    assert_eq!(format_time_of_day(GAME_DAY_SECONDS), "00:00");
    assert_eq!(advance_time_of_day(GAME_DAY_SECONDS - 1.0, 1.0, 1.0), 0.0);
}

#[test]
fn sky_gradient_has_distinct_day_and_night_palettes() {
    let night = sky_vertex_colors(0.0);
    let day = sky_vertex_colors(12.0 * 60.0 * 60.0);
    let dawn = sky_vertex_colors(6.0 * 60.0 * 60.0);

    assert_eq!(night.len(), 33 * 65);
    assert_ne!(night, day);
    assert_ne!(dawn, day);
    assert!(night
        .iter()
        .chain(&day)
        .chain(&dawn)
        .all(|color| color.iter().all(|channel| (0.0..=1.0).contains(channel))));

    let noon_direction = sun_direction(12.0 * 60.0 * 60.0);
    let midnight_direction = sun_direction(0.0);
    assert!(noon_direction.y > 0.99);
    assert!(midnight_direction.y < -0.99);
}

#[test]
fn world_save_rejects_unsupported_versions_and_invalid_chunks() {
    let mut save = WorldSave {
        version: SAVE_VERSION + 1,
        name: "Prueba".into(),
        seed: 1,
        terrain_version: TERRAIN_VERSION,
        time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
        player_position: [0.0, 0.0, 0.0],
        player_yaw: 0.0,
        player_pitch: 0.0,
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
    };
    assert!(validate_world_save(&save).is_err());

    save.version = SAVE_VERSION;
    save.chunks.push(ChunkSave {
        coord: [0, 0, 0],
        blocks: vec![AIR; CHUNK_VOLUME - 1],
    });
    assert!(validate_world_save(&save).is_err());
}

#[test]
fn different_food_types_have_correct_saturation() {
    let saturations = [
        (FoodType::BerryRation, 4.0),
        (FoodType::Apple, 6.0),
        (FoodType::Mushroom, 2.0),
        (FoodType::Wheat, 8.0),
        (FoodType::RawMeat, 3.0),
        (FoodType::CookedMeat, 8.0),
    ];

    for (food_type, expected_saturation) in saturations {
        assert_eq!(
            food_type.saturation(),
            expected_saturation,
            "Food {:?} should have saturation {:?}",
            food_type,
            expected_saturation
        );
    }
}

#[test]
fn poisonous_mushrooms_cause_damage() {
    let mut survival = Survival::default();
    survival.food = 5;
    survival.hunger = 5.0;
    let initial_health = survival.health;

    eat_food_of_type(&mut survival, FoodType::Mushroom);

    assert_eq!(survival.food, 4, "Food should be consumed");
    assert!(
        survival.hunger > 5.0,
        "Hunger should increase even when poisoned"
    );
    assert!(
        survival.health < initial_health,
        "Poisonous mushroom should cause damage"
    );
}

#[test]
fn cooked_meat_provides_more_saturation_than_raw_meat() {
    assert!(FoodType::CookedMeat.saturation() > FoodType::RawMeat.saturation());
    assert_eq!(FoodType::CookedMeat.saturation(), 8.0);
    assert_eq!(FoodType::RawMeat.saturation(), 3.0);
}

#[test]
fn eating_different_foods_increases_hunger_differently() {
    let mut survival1 = Survival::default();
    survival1.food = 10;
    survival1.hunger = 5.0;

    let mut survival2 = Survival::default();
    survival2.food = 10;
    survival2.hunger = 5.0;

    eat_food_of_type(&mut survival1, FoodType::Apple);
    eat_food_of_type(&mut survival2, FoodType::Wheat);

    assert!(
        survival2.hunger > survival1.hunger,
        "Wheat should restore more hunger than apple"
    );
    assert_eq!(survival1.hunger, 11.0, "Apple restores 6 hunger");
    assert_eq!(survival2.hunger, 13.0, "Wheat restores 8 hunger");
}

#[test]
fn doors_block_player_collision_when_closed_but_allow_passage_when_open() {
    let mut world = WorldData {
        seed: 42,
        ..default()
    };
    world
        .chunks
        .insert(IVec3::ZERO, generate_chunk(IVec3::ZERO, 42));

    let door_pos = IVec3::new(0, 0, 0);
    {
        let chunk = world.chunks.get_mut(&IVec3::ZERO).unwrap();
        chunk.set(door_pos, WOODEN_DOOR);
    }
    world.door_states.insert(door_pos, DoorState::Closed);

    // Closed door should be solid
    assert!(
        is_block_solid(&world, door_pos),
        "Closed door blocks collision"
    );

    // Open door should not be solid
    world.door_states.insert(door_pos, DoorState::Open);
    assert!(
        !is_block_solid(&world, door_pos),
        "Open door allows passage"
    );
}

#[test]
fn stairs_and_slabs_do_not_block_collision_but_render() {
    let mut world = WorldData {
        seed: 42,
        ..default()
    };
    world
        .chunks
        .insert(IVec3::ZERO, generate_chunk(IVec3::ZERO, 42));

    let stair_pos = IVec3::new(1, 0, 0);
    let slab_pos = IVec3::new(2, 0, 0);
    {
        let chunk = world.chunks.get_mut(&IVec3::ZERO).unwrap();
        chunk.set(stair_pos, STAIRS);
        chunk.set(slab_pos, SLAB);
    }

    // Stairs and slabs should not be solid (player can walk through them)
    assert!(!is_block_solid(&world, stair_pos), "Stairs allow movement");
    assert!(!is_block_solid(&world, slab_pos), "Slabs allow movement");

    // But they render in the world (chunk has the blocks)
    let chunk = world.chunks.get(&IVec3::ZERO).unwrap();
    assert_eq!(chunk.get(stair_pos), STAIRS);
    assert_eq!(chunk.get(slab_pos), SLAB);
}

#[test]
fn furnace_opens_on_use_smelts_and_spills_when_broken() {
    use crate::furnace::SMELT_SECONDS;

    let raw = Item::Mineral(MineralKind::RawCopper);
    let ingot = Item::Mineral(MineralKind::CopperIngot);
    let coal = Item::Mineral(MineralKind::Coal);
    let (mut app, _, _) = mining_test_app(FURNACE, CraftedTools::default());
    app.add_systems(
        Update,
        (inventory_controls, inventory_clicks, tick_furnaces)
            .chain()
            .before(edit_blocks),
    );
    app.init_resource::<ButtonInput<KeyCode>>();
    app.world_mut().spawn((Node::default(), InventoryPanel));
    let input = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::FurnaceInput))
        .id();
    let fuel = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::FurnaceFuel))
        .id();
    let output = app
        .world_mut()
        .spawn((Interaction::None, InventoryTarget::FurnaceOutput))
        .id();
    let position = IVec3::new(0, 1, 2);

    // Using the furnace opens its screen on the following frame.
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    mining_tick(&mut app, 0.0);
    mining_tick(&mut app, 0.0);
    assert_eq!(app.world().resource::<CraftingUi>().furnace, Some(position));
    assert!(app.world().resource::<CraftingUi>().is_active());

    let click = |app: &mut App, entity: Entity, stack: Option<ItemStack>| {
        if stack.is_some() {
            app.world_mut().resource_mut::<Inventory>().cursor = stack;
        }
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        mining_tick(app, 0.0);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset_all();
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
    };
    click(
        &mut app,
        input,
        Some(ItemStack {
            item: raw,
            count: 3,
        }),
    );
    click(
        &mut app,
        fuel,
        Some(ItemStack {
            item: coal,
            count: 2,
        }),
    );
    assert!(app.world().resource::<Inventory>().cursor.is_none());

    mining_tick(&mut app, SMELT_SECONDS * 2.0);
    let furnace = app.world().resource::<WorldData>().furnaces[&position].clone();
    assert_eq!(
        furnace.output,
        Some(ItemStack {
            item: ingot,
            count: 2
        })
    );
    assert!(furnace_status(&furnace).starts_with("Fuego"));

    // The output slot only gives items; a held item cannot be stored there.
    click(
        &mut app,
        output,
        Some(ItemStack {
            item: coal,
            count: 1,
        }),
    );
    assert_eq!(
        app.world().resource::<WorldData>().furnaces[&position].output,
        furnace.output
    );
    app.world_mut().resource_mut::<Inventory>().cursor = None;
    click(&mut app, output, None);
    assert_eq!(app.world().resource::<Inventory>().cursor, furnace.output);
    assert_eq!(
        app.world().resource::<WorldData>().furnaces[&position].output,
        None
    );

    // The furnace state survives a save round trip.
    let save = {
        let world = app.world();
        snapshot_world(
            world.resource::<WorldData>(),
            Vec3::ZERO,
            &Player {
                yaw: 0.0,
                pitch: 0.0,
                velocity: Vec3::ZERO,
                grounded: false,
                flying: false,
                jump_buffer: 0.0,
                coyote_time: 0.0,
                step_up_remaining: 0.0,
                jump_key_down: false,
                flight_key_down: false,
            },
            &Inventory::default(),
            GRASS,
            0,
            0.0,
            &Survival::default(),
            &CraftedTools::default(),
        )
    };
    let loaded: WorldSave = ron::from_str(&ron::to_string(&save).unwrap()).unwrap();
    assert_eq!(loaded.furnaces.len(), 1);
    assert_eq!(loaded.furnaces[0].position, position.to_array());
    assert_eq!(
        loaded.furnaces[0].furnace,
        app.world().resource::<WorldData>().furnaces[&position]
    );

    // Closing the screen and breaking the furnace drops what it still holds.
    app.world_mut().resource_mut::<Inventory>().cursor = None;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    mining_tick(&mut app, 0.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    assert!(!app.world().resource::<CraftingUi>().is_active());
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, 10.0);
    assert_eq!(block_at(app.world().resource::<WorldData>(), position), AIR);
    assert!(app.world().resource::<WorldData>().furnaces.is_empty());
    assert_eq!(dropped_items(&mut app, Item::Block(FURNACE)), 1);
    // The last ore finished smelting while the block was being mined.
    assert_eq!(dropped_items(&mut app, raw), 0);
    assert_eq!(dropped_items(&mut app, ingot), 1);
    assert_eq!(dropped_items(&mut app, coal), 1);
}

#[test]
fn furnace_saves_are_validated_and_old_saves_load_without_furnaces() {
    use crate::furnace::Furnace;

    let mut save = snapshot_world(
        &WorldData {
            name: "Hornos".into(),
            ..default()
        },
        Vec3::ZERO,
        &Player {
            yaw: 0.0,
            pitch: 0.0,
            velocity: Vec3::ZERO,
            grounded: false,
            flying: false,
            jump_buffer: 0.0,
            coyote_time: 0.0,
            step_up_remaining: 0.0,
            jump_key_down: false,
            flight_key_down: false,
        },
        &Inventory::default(),
        GRASS,
        0,
        0.0,
        &Survival::default(),
        &CraftedTools::default(),
    );
    validate_world_save(&save).unwrap();

    // Saves written before furnaces existed have no such field.
    let encoded = ron::to_string(&save).unwrap();
    let start = encoded.find("furnaces:").unwrap();
    let legacy = format!("{})", encoded[..start].trim_end_matches(','));
    let loaded: WorldSave = ron::from_str(&legacy).unwrap();
    assert!(loaded.furnaces.is_empty());
    validate_world_save(&loaded).unwrap();

    save.furnaces.push(FurnaceSave {
        position: [1, 2, 3],
        furnace: Furnace {
            progress: f32::NAN,
            ..default()
        },
    });
    assert!(validate_world_save(&save).is_err());
    save.furnaces[0].furnace.progress = 0.0;
    validate_world_save(&save).unwrap();
    save.furnaces.push(FurnaceSave {
        position: [1, 2, 3],
        furnace: Furnace::default(),
    });
    assert!(validate_world_save(&save).is_err());
}

#[test]
fn iron_ore_needs_a_copper_pickaxe_and_better_tools_mine_faster() {
    let pickaxe = |material| {
        Some(ItemStack {
            item: Item::Tool {
                kind: ToolKind::Pickaxe,
                durability: 10,
                material,
            },
            count: 1,
        })
    };
    let position = IVec3::new(0, 1, 2);
    let hardness = block_definition(IRON_ORE).hardness;

    let (mut app, _, _) = mining_test_app(IRON_ORE, CraftedTools::default());
    app.world_mut().resource_mut::<Inventory>().slots[0] = pickaxe(ToolMaterial::Stone);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    mining_tick(&mut app, hardness * 2.0);
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), position),
        IRON_ORE
    );
    assert!(app.world().resource::<MiningState>().target.is_none());
    assert!(app
        .world()
        .resource::<Survival>()
        .notice
        .contains("mejor material"));
    assert_eq!(
        app.world().resource::<Inventory>().slots[0],
        pickaxe(ToolMaterial::Stone)
    );

    app.world_mut().resource_mut::<Inventory>().slots[0] = pickaxe(ToolMaterial::Copper);
    mining_tick(&mut app, hardness / 3.0 - 0.05);
    assert_eq!(
        block_at(app.world().resource::<WorldData>(), position),
        IRON_ORE
    );
    mining_tick(&mut app, 0.1);
    assert_eq!(block_at(app.world().resource::<WorldData>(), position), AIR);
    assert!(dropped_items(&mut app, Item::Mineral(MineralKind::RawIron)) >= 1);

    assert_eq!(
        mining_duration_with(STONE, Some(MiningTool::Pickaxe), ToolMaterial::Iron),
        block_definition(STONE).hardness / 4.0
    );
    assert_eq!(
        mining_duration_with(STONE, None, ToolMaterial::Iron),
        block_definition(STONE).hardness
    );
    assert!(
        ToolMaterial::Stone < ToolMaterial::Copper && ToolMaterial::Copper < ToolMaterial::Iron
    );
    assert!(ToolMaterial::ALL
        .windows(2)
        .all(|pair| pair[0].sword_damage() < pair[1].sword_damage()));
}

#[test]
fn only_full_opaque_cubes_hide_the_faces_behind_them() {
    for block in [STONE, DIRT, GRASS, SAND, WOOD, PLANKS, FURNACE, IRON_ORE] {
        assert!(block_occludes(block), "block {block}");
    }
    for block in [
        AIR,
        WATER,
        LAVA,
        LEAVES,
        APPLE,
        TORCH,
        TALL_GRASS,
        RED_FLOWER,
        YELLOW_FLOWER,
    ] {
        assert!(!block_occludes(block), "block {block}");
    }

    // Counts the quads lying on the plane `axis == value`.
    let quads_on_plane = |blocks: &[(IVec3, u8)], axis: usize, value: f32| {
        let mut chunk = Chunk::new();
        for &(position, block) in blocks {
            chunk.set(position, block);
        }
        let world = WorldData {
            chunks: HashMap::from([(IVec3::ZERO, chunk)]),
            ..default()
        };
        let mesh = rendering::build_chunk_mesh(&world, IVec3::ZERO);
        let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("block mesh positions must be 3D floats");
        };
        positions
            .chunks_exact(4)
            .filter(|quad| quad.iter().all(|vertex| vertex[axis] == value))
            .count()
    };
    let base = IVec3::new(4, 4, 4);
    let above = base + IVec3::Y;
    let beside = base + IVec3::X;

    // The ground stays visible under plants, torches and leaves.
    for cover in [TALL_GRASS, RED_FLOWER, TORCH, LEAVES] {
        // Exactly the ground's top face: the cover's own underside is
        // hidden by the opaque ground.
        let ground_top = quads_on_plane(&[(base, GRASS), (above, cover)], 1, 5.0);
        assert_eq!(ground_top, 1, "cover {cover}");
    }
    // Opaque neighbours still hide each other.
    assert_eq!(quads_on_plane(&[(base, STONE), (above, DIRT)], 1, 5.0), 0);
    assert_eq!(quads_on_plane(&[(base, STONE), (beside, STONE)], 0, 5.0), 0);
    // Blocks of the same see-through kind share no inner faces.
    assert_eq!(
        quads_on_plane(&[(base, LEAVES), (beside, LEAVES)], 0, 5.0),
        0
    );
    assert_eq!(quads_on_plane(&[(base, WATER), (beside, WATER)], 0, 5.0), 0);
    // A solid face under water is drawn; the water face against it is not.
    assert_eq!(quads_on_plane(&[(base, STONE), (above, WATER)], 1, 5.0), 1);
}
