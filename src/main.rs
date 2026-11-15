use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use rand::RngExt;
use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter::once;

const SQUARE_HOVER_FACTOR: i32 = 1;

#[derive(Component)]
struct LayerHud;

fn setup_hud(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        LayerHud,
        Text::new(""),
        TextFont {
            font: asset_server.load("fonts/JetBrainsMono-Medium.ttf").into(),
            font_size: FontSize::Px(24.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(20),
            left: px(20),
            ..default()
        },
        Pickable::IGNORE,
    ));

    commands.spawn((
        HoverHud,
        Text::new(""),
        TextFont {
            font: asset_server.load("fonts/JetBrainsMono-Thin.ttf").into(),
            font_size: FontSize::Px(24.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(49),
            left: px(80),
            ..default()
        },
        Pickable::IGNORE,
    ));

    commands.spawn((
        PuzzleHud,
        Text::new(""),
        TextFont {
            font: asset_server.load("fonts/JetBrainsMono-Thin.ttf").into(),
            font_size: FontSize::Px(24.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(279),
            left: px(80),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

fn update_layer_hud(game: Res<GameData>, mut hud: Query<&mut Text, With<LayerHud>>) {
    if !game.is_changed() {
        return;
    }

    let lines = once("Bits:".to_string())
        .chain(
            (0..6).map(|layer| format!("{} {layer}", if layer == game.layer { "▶" } else { " " })),
        )
        .chain(once(format!("\nTarget:",)))
        .chain((0..6).map(|bit| format!("  {}", (game.target >> bit) % 2)))
        .collect::<Vec<_>>()
        .join("\n");

    for mut text in &mut hud {
        text.0 = lines.clone();
    }
}

#[derive(Resource, Default)]
struct DrawNumbers(bool);

#[derive(Resource, Default)]
struct HoveredTile(Option<Entity>);

#[derive(Component)]
struct HoverHud;

#[derive(Component)]
struct PuzzleHud;

fn remember_hover(over: On<PointerOver>, mut hovered: ResMut<HoveredTile>) {
    if over.pointer.id.is_mouse() {
        hovered.0 = Some(over.event_target());
    }
}

fn forget_hover(out: On<PointerOut>, mut hovered: ResMut<HoveredTile>) {
    if out.pointer.id.is_mouse() && hovered.0 == Some(out.event_target()) {
        hovered.0 = None;
    }
}

fn update_hover_hud(
    hovered: Res<HoveredTile>,
    game: Res<GameData>,
    squares: Query<&Square>,
    position: Query<&PositionData>,
    mut hud: Query<&mut Text, (With<HoverHud>, Without<PuzzleHud>)>,
    mut puzzle_hud: Query<&mut Text, (With<PuzzleHud>, Without<HoverHud>)>,
) {
    if !hovered.is_changed() && !game.is_changed() {
        return;
    }

    let message = hovered
        .0
        .and_then(|entity| squares.get(entity).ok())
        .map(|square| {
            (0..6)
                .map(|bit| format!("{}", game.values[bit][square.y][square.z] as usize))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "".to_string());

    for mut text in &mut hud {
        text.0 = message.clone();
    }

    let message = hovered
        .0
        .and_then(|entity| position.get(entity).ok())
        .filter(|position| position.location.y < 0.0)
        .map(|position| {
            puzzle_tile_bits(
                &game,
                position.location.x as usize,
                position.location.z as usize,
            )
        })
        .unwrap_or_default();

    for mut text in &mut puzzle_hud {
        text.0 = message.clone();
    }
}

fn puzzle_tile_bits(game: &GameData, x: usize, z: usize) -> String {
    (0..6)
        .map(|bit| format!("{}", game.values[bit][z][x] as usize))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Component, Reflect, Clone, Copy)]
struct PositionData {
    location: Vec3,
}

#[derive(Component, Copy, Clone)]
struct Square {
    x: usize,
    y: usize,
    z: usize,
}

#[derive(Component)]
struct TileLabel {
    tile: Entity,
    text: Entity,
}

#[derive(Component, Clone)]
struct OriginalMaterial(Handle<StandardMaterial>, Handle<StandardMaterial>);

#[derive(Clone)]
struct HighlightKey {
    handle: Handle<StandardMaterial>,
    variant: i32,
}
impl PartialEq for HighlightKey {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant && self.handle == other.handle
    }
}
impl Eq for HighlightKey {}
impl Hash for HighlightKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
        self.variant.hash(state);
    }
}

#[derive(Resource, Default)]
struct HighlightCache(HashMap<HighlightKey, Handle<StandardMaterial>>);

#[derive(Resource)]
struct GameData {
    layer: usize,
    values: Vec<Vec<Vec<bool>>>,
    puzzle: Vec<(usize, usize)>,
    target: u8,
}

impl Default for GameData {
    fn default() -> Self {
        let mut rng: rand::rngs::SmallRng = rand::make_rng();
        let target = rng.random::<u8>() % 64;
        let count = rng.random::<u8>() % 65;
        let all = (0..8)
            .flat_map(|i| (0..8).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let puzzle = all
            .sample(&mut rng, count as usize)
            .cloned()
            .collect::<Vec<_>>();
        let mut values = vec![vec![vec![false; 8]; 8]; 8];
        for power in 0..6 {
            for x in 0..8 {
                for y in 0..8 {
                    let bit = (x * 8 + y) / (1 << power);
                    if bit % 2 == 1 {
                        values[power][x][y] = true;
                    }
                }
            }
        }
        Self {
            layer: 0,
            values,
            puzzle,
            target,
        }
    }
}

fn update_layer_visibility(
    game_data: Res<GameData>,
    mut query: Query<(&PositionData, &mut Visibility)>,
) {
    // Only execute state mutations when game data changes to minimize resource consumption
    if !game_data.is_changed() {
        return;
    }

    for (pos, mut visibility) in &mut query {
        if pos.location.y < 0.0 {
            continue;
        }

        let layer_index = pos.location.y as usize;

        // Hide layers situated strictly above the currently active layer
        if layer_index < game_data.layer {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

fn process_mouse_click(
    click: On<PointerClick>,
    mut game_data: ResMut<GameData>,
    mut cache: ResMut<HighlightCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tiles: Query<(
        &PositionData,
        &OriginalMaterial,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if let Ok((pos, original, mut displayed)) = tiles.get_mut(click.event_target()) {
        if pos.location.y >= 0.0 {
            let value = &mut game_data.values[pos.location.y as usize][pos.location.z as usize]
                [pos.location.x as usize];
            value.toggle();

            let base = if *value { &original.0 } else { &original.1 };
            displayed.0 = highlight_for(base, &mut cache, &mut materials, SQUARE_HOVER_FACTOR);
        }
    }
}

fn update_chosen_tile(
    game: Res<GameData>,
    mut cache: ResMut<HighlightCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tiles: Query<(
        &PositionData,
        &OriginalMaterial,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if game.is_changed() {
        let mut total = 0;
        for power in 0..6 {
            if layer_parity(&game, power) {
                total += 1 << power;
            }
        }
        let x = total % 8;
        let y = total / 8;
        for (position, original_material, mut material) in tiles {
            if position.location.y < 0.0 {
                if position.location.x == x as f32 && position.location.z == y as f32 {
                    material.0 = highlight_for(&mut material, &mut cache, &mut materials, 2)
                } else {
                    material.0 = original_material.0.clone();
                }
            }
        }
    }
}

fn layer_parity(game: &Res<GameData>, layer: usize) -> bool {
    let mut count = 0;
    for (i, j) in &game.puzzle {
        if game.values[layer][*j][*i] {
            count += 1;
        }
    }
    count % 2 == 1
}

fn update_tile_labels(
    game: Res<GameData>,
    draw_numbers: Res<DrawNumbers>,
    ui_scale: Res<UiScale>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    tiles: Query<(&Square, &GlobalTransform)>,
    mut labels: Query<(&TileLabel, &mut Node)>,
    mut texts: Query<&mut Text>,
) {
    if !draw_numbers.0 {
        for (_, mut node) in &mut labels {
            node.display = Display::None;
        }
        return;
    }

    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    for (label, mut node) in &mut labels {
        let Ok((square, tile_transform)) = tiles.get(label.tile) else {
            node.display = Display::None;
            continue;
        };

        if square.x != game.layer {
            node.display = Display::None;
            continue;
        }

        // The cuboid's local +Z face becomes the top after its rotation.
        let top_center = tile_transform.transform_point(Vec3::new(0.0, 0.0, 0.201));
        let Ok(screen) = camera.world_to_viewport(camera_transform, top_center) else {
            node.display = Display::None;
            continue;
        };

        // Center the 80×32 UI container on the projected point.
        node.display = Display::Flex;
        node.left = px(screen.x / ui_scale.0 - 40.0);
        node.top = px(screen.y / ui_scale.0 - 16.0);

        let number: usize = (0..8)
            .map(|bit| (game.values[bit][square.y][square.z] as usize) << bit)
            .sum();

        if let Ok(mut text) = texts.get_mut(label.text) {
            let value = number.to_string();
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}

fn highlight_for(
    orig: &Handle<StandardMaterial>,
    cache: &mut HighlightCache,
    materials: &mut Assets<StandardMaterial>,
    variant: i32,
) -> Handle<StandardMaterial> {
    let key = HighlightKey {
        handle: orig.clone(),
        variant,
    };
    if let Some(h) = cache.0.get(&key) {
        return h.clone();
    }

    let mut mat = materials
        .get(orig)
        .cloned()
        .unwrap_or_else(StandardMaterial::default);

    // Brighter regardless of texture
    // mat.base_color = lighten_towards_white(mat.base_color, 0.22);

    if variant == 1 {
        mat.base_color = Color::from(RED_200);
        mat.emissive = LinearRgba::rgb(0.3, 0.0, 0.0);
    } else if variant == 2 {
        mat.base_color = Color::from(BLUE_200);
        mat.emissive = LinearRgba::rgb(0.0, 0.0, 0.3);
    } else {
        mat.base_color = Color::from(GREEN_200);
        mat.emissive = LinearRgba::rgb(0.0, 0.3, 0.0);
    }

    let hi = materials.add(mat);
    cache.0.insert(key, hi.clone());
    hi
}

fn swap_to_highlight_on<E: bevy::prelude::EntityEvent>(
    variant: i32,
) -> impl Fn(
    On<E>,
    ResMut<HighlightCache>,
    ResMut<Assets<StandardMaterial>>,
    Res<GameData>,
    Query<&mut MeshMaterial3d<StandardMaterial>>,
    Query<&OriginalMaterial>,
    Query<&PositionData>,
) {
    move |trigger, mut cache, mut materials, game_data, mut mut_mat_q, orig_q, pos_data| {
        if let (Ok(mut mat), Ok(orig), Ok(pos_data)) = (
            mut_mat_q.get_mut(trigger.event_target()), // ****
            orig_q.get(trigger.event_target()),
            pos_data.get(trigger.event_target()),
        ) {
            let base_mat = if game_data.values[pos_data.location.y as usize]
                [pos_data.location.z as usize][pos_data.location.x as usize]
            {
                &orig.0
            } else {
                &orig.1
            };
            let hi = highlight_for(base_mat, &mut cache, &mut materials, variant);
            mat.0 = hi;
        }
    }
}

fn swap_back_on<E: bevy::prelude::EntityEvent>() -> impl Fn(
    On<E>,
    Res<GameData>,
    Query<&mut MeshMaterial3d<StandardMaterial>>,
    Query<&OriginalMaterial>,
    Query<&PositionData>,
) {
    move |trigger, game_data, mut mut_mat_q, orig_q, pos_data| {
        if let (Ok(mut mat), Ok(orig), Ok(pos_data)) = (
            mut_mat_q.get_mut(trigger.event_target()),
            orig_q.get(trigger.event_target()),
            pos_data.get(trigger.event_target()),
        ) {
            let base_mat = if game_data.values[pos_data.location.y as usize]
                [pos_data.location.z as usize][pos_data.location.x as usize]
            {
                &orig.0
            } else {
                &orig.1
            };
            mat.0 = base_mat.clone();
        }
    }
}

pub fn load_wooden_materials(
    asset_server: &Res<AssetServer>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
    let dark_walnut = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/dark_walnut_base_color.jpg")),
        metallic_roughness_texture: Some(
            asset_server.load("textures/dark_walnut_metallic_roughness.png"),
        ),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        clearcoat: 1.0,
        clearcoat_perceptual_roughness: 0.04,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let light_wood = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/light_wood_base_color.jpg")),
        metallic_roughness_texture: Some(
            asset_server.load("textures/light_wood_metallic_roughness.png"),
        ),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        clearcoat: 1.0,
        clearcoat_perceptual_roughness: 0.04,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    (dark_walnut, light_wood)
}

fn create_squares(
    commands: &mut Commands,
    game_data: &ResMut<GameData>,
    mut cache: ResMut<HighlightCache>,
    asset_server: &Res<AssetServer>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
) {
    let (mat1, mat2) = load_wooden_materials(asset_server, materials);

    for i in 0..8 {
        for j in 0..8 {
            for k in 0..6 {
                let location = Vec3::new(i as f32, k as f32, j as f32);
                let mesh_handle = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 0.4)));

                let base_mat = if game_data.values[k][j][i] {
                    mat1.clone()
                } else {
                    mat2.clone()
                };

                let tile = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(base_mat),
                        Transform::from_rotation(Quat::from_rotation_x(
                            -std::f32::consts::FRAC_PI_2,
                        ))
                        .with_translation(Vec3 {
                            x: i as f32,
                            y: -0.2 - k as f32 / 2.0,
                            z: j as f32,
                        }),
                        PositionData { location },
                        OriginalMaterial(mat1.clone(), mat2.clone()),
                        Square { x: k, y: j, z: i },
                    ))
                    .observe(swap_to_highlight_on::<PointerOver>(SQUARE_HOVER_FACTOR))
                    .observe(swap_back_on::<PointerOut>())
                    .observe(remember_hover)
                    .observe(forget_hover)
                    .observe(process_mouse_click)
                    .id();

                let text = commands
                    .spawn((
                        Text::new("0"),
                        TextFont {
                            font: asset_server.load("fonts/JetBrainsMono-Thin.ttf").into(),
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Pickable::IGNORE,
                    ))
                    .id();

                commands
                    .spawn((
                        TileLabel { tile, text },
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(80),
                            height: px(32),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            display: Display::None,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .add_child(text);
            }
        }
    }

    for i in 0..8 {
        for j in 0..8 {
            let location = Vec3::new(i as f32, -1.0, j as f32);
            let mesh_handle = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 0.4)));

            let mut base_mat = if (i + j) % 2 == 0 {
                mat1.clone()
            } else {
                mat2.clone()
            };

            if i * 8 + j == game_data.target {
                base_mat = highlight_for(&mut base_mat, &mut cache, materials, 3);
            }

            commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(base_mat.clone()),
                    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                        .with_translation(Vec3 {
                            x: i as f32,
                            y: -0.2,
                            z: j as f32 - 10.0,
                        }),
                    PositionData { location },
                    OriginalMaterial(base_mat.clone(), base_mat.clone()),
                ))
                .observe(remember_hover)
                .observe(forget_hover)
                .observe(process_mouse_click);

            if game_data.puzzle.contains(&(i as usize, j as usize)) {
                let mesh_handle = meshes.add(Mesh::from(Extrusion::new(Circle::new(0.35), 0.2)));

                commands.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: AMBER_200.into(),
                        metallic: 0.5,
                        ..Default::default()
                    })),
                    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                        .with_translation(Vec3 {
                            x: i as f32,
                            y: 0.0,
                            z: j as f32 - 10.0,
                        }),
                ));
            }
        }
    }
}

fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_data: ResMut<GameData>,
    mut draw_numbers: ResMut<DrawNumbers>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Minus) {
        if game_data.layer > 0 {
            game_data.layer -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::Equal) {
        if game_data.layer < 5 {
            game_data.layer += 1;
        }
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        draw_numbers.0.toggle()
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_data: ResMut<GameData>,
    cache: ResMut<HighlightCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    setup_hud(&mut commands, &asset_server);
    create_squares(
        &mut commands,
        &game_data,
        cache,
        &asset_server,
        &mut materials,
        &mut meshes,
    );

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    commands.spawn((
        Camera3d { ..default() },
        Transform::from_xyz(12., 7., 4.).looking_at(
            Vec3 {
                x: 4.,
                y: 0.,
                z: 4.,
            },
            Vec3::Y,
        ),
        PanOrbitCamera {
            button_orbit: MouseButton::Right,
            focus: Vec3::new(4.0, 0.0, 4.0),
            ..default()
        },
    ));
}

fn pan_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<(&Transform, &mut PanOrbitCamera)>,
) {
    let mut input = Vec2::ZERO;
    if keyboard.pressed(KeyCode::ArrowLeft) {
        input.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        input.x += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        input.y += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        input.y -= 1.0;
    }

    if input == Vec2::ZERO {
        return;
    }
    input = input.normalize();

    for (transform, mut camera) in &mut cameras {
        if !camera.initialized {
            continue;
        }
        let right = transform.rotation * Vec3::X;
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let forward = Vec3::Y.cross(right);
        camera.target_focus += (right * input.x + forward * input.y) * 5.0 * time.delta_secs();
        camera.force_update = true;
    }
}

fn main() {
    App::new()
        // Set WindowPlugin to change title and size
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bitflips".into(),
                resolution: bevy::window::WindowResolution::new(1600, 1600),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .init_resource::<HoveredTile>()
        .init_resource::<DrawNumbers>()
        .insert_resource(GameData::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(HighlightCache::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                keyboard_input_system,
                update_layer_visibility,
                update_layer_hud,
                update_hover_hud,
                pan_camera,
                update_chosen_tile,
            ),
        )
        .add_systems(
            PostUpdate,
            update_tile_labels
                .after(TransformSystems::Propagate)
                .before(bevy::ui::UiSystems::Content),
        )
        .run();
}
