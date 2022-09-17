use bevy::prelude::*;
use clap::Parser;

mod debug;

mod card;
mod game_state;

mod app_state_in_game;
mod app_state_picking_hands;
mod app_state_start_menu;

const RENDER_SIZE: Vec2 = Vec2::new(320., 240.);
const RENDER_HSIZE: Vec2 = Vec2::new(RENDER_SIZE.x / 2., RENDER_SIZE.y / 2.);
const SCREEN_SIZE: Vec2 = Vec2::new(RENDER_SIZE.x * 4., RENDER_SIZE.y * 4.);

const CARD_SIZE: Vec2 = Vec2::new(42., 51.);
const COIN_SIZE: Vec2 = Vec2::new(40., 40.);

// color picked from the background.png file
const CLEAR_COLOR: Color = Color::rgb(0.03137255, 0.03137255, 0.03137255);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Initialization,
    StartMenu,
    PickingHands,
    InGame,
}

#[derive(Debug, Parser)]
struct Args {
    implementation: String,
}

fn main() {
    App::new()
        .insert_resource(bevy::render::texture::ImageSettings::default_nearest())
        .insert_resource(WindowDescriptor {
            title: "Tetra Master".to_string(),
            width: SCREEN_SIZE.x,
            height: SCREEN_SIZE.y,
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: RENDER_SIZE.x,
                min_height: RENDER_SIZE.y,
                ..default()
            },
            ..default()
        })
        .insert_resource(Args::parse())
        .insert_resource(AppAssets::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(debug::Plugin)
        .add_plugin(card::Plugin)
        .add_plugin(app_state_start_menu::Plugin)
        .add_plugin(app_state_picking_hands::Plugin)
        .add_plugin(app_state_in_game::Plugin)
        .add_state(AppState::Initialization)
        .add_system_set(SystemSet::on_enter(AppState::Initialization).with_system(setup))
        .run();
}

fn cleanup<T: Component>(mut commands: Commands, entities: Query<Entity, With<T>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Default)]
struct AppAssets {
    font: Handle<Font>,
    background: Handle<Image>,
    board: Handle<Image>,
    coin_flip: Handle<TextureAtlas>,
    card_bg_gray: Handle<Image>,
    card_bg_blue: Handle<Image>,
    card_bg_red: Handle<Image>,
    card_arrow_up: Handle<Image>,
    card_arrow_up_right: Handle<Image>,
    card_arrow_right: Handle<Image>,
    card_arrow_down_right: Handle<Image>,
    card_arrow_down: Handle<Image>,
    card_arrow_down_left: Handle<Image>,
    card_arrow_left: Handle<Image>,
    card_arrow_up_left: Handle<Image>,
    card_faces: Handle<TextureAtlas>,
    card_stat_font: Handle<TextureAtlas>,
    card_counter_center: Handle<Image>,
    card_counter_blue: [Handle<Image>; 11],
    card_counter_red: [Handle<Image>; 11],
}

fn setup(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut app_assets: ResMut<AppAssets>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    // load assets
    app_assets.font = asset_server.load("alexandria.ttf");

    app_assets.background = asset_server.load("background.png");
    app_assets.board = asset_server.load("board.png");

    app_assets.coin_flip = {
        let handle = asset_server.load("coin-flip.png");
        let atlas = TextureAtlas::from_grid(handle, COIN_SIZE, 8, 1);
        texture_atlases.add(atlas)
    };

    app_assets.card_bg_gray = asset_server.load("card-bg-gray.png");
    app_assets.card_bg_blue = asset_server.load("card-bg-blue.png");
    app_assets.card_bg_red = asset_server.load("card-bg-red.png");

    app_assets.card_arrow_up = asset_server.load("card-arrow-up.png");
    app_assets.card_arrow_up_right = asset_server.load("card-arrow-up-right.png");
    app_assets.card_arrow_right = asset_server.load("card-arrow-right.png");
    app_assets.card_arrow_down_right = asset_server.load("card-arrow-down-right.png");
    app_assets.card_arrow_down = asset_server.load("card-arrow-down.png");
    app_assets.card_arrow_down_left = asset_server.load("card-arrow-down-left.png");
    app_assets.card_arrow_left = asset_server.load("card-arrow-left.png");
    app_assets.card_arrow_up_left = asset_server.load("card-arrow-up-left.png");

    app_assets.card_faces = {
        let handle = asset_server.load("card-faces.png");
        let atlas = TextureAtlas::from_grid(handle, CARD_SIZE, 10, 10);
        texture_atlases.add(atlas)
    };

    app_assets.card_stat_font = {
        let handle = asset_server.load("card-stat-font.png");
        let atlas = TextureAtlas::from_grid(handle, (7., 7.).into(), 19, 1);
        texture_atlases.add(atlas)
    };

    app_assets.card_counter_center = asset_server.load("card-counter-center.png");
    app_assets.card_counter_blue = [
        asset_server.load("card-counter-blue-00.png"),
        asset_server.load("card-counter-blue-01.png"),
        asset_server.load("card-counter-blue-02.png"),
        asset_server.load("card-counter-blue-03.png"),
        asset_server.load("card-counter-blue-04.png"),
        asset_server.load("card-counter-blue-05.png"),
        asset_server.load("card-counter-blue-06.png"),
        asset_server.load("card-counter-blue-07.png"),
        asset_server.load("card-counter-blue-08.png"),
        asset_server.load("card-counter-blue-09.png"),
        asset_server.load("card-counter-blue-10.png"),
    ];
    app_assets.card_counter_red = [
        asset_server.load("card-counter-red-00.png"),
        asset_server.load("card-counter-red-01.png"),
        asset_server.load("card-counter-red-02.png"),
        asset_server.load("card-counter-red-03.png"),
        asset_server.load("card-counter-red-04.png"),
        asset_server.load("card-counter-red-05.png"),
        asset_server.load("card-counter-red-06.png"),
        asset_server.load("card-counter-red-07.png"),
        asset_server.load("card-counter-red-08.png"),
        asset_server.load("card-counter-red-09.png"),
        asset_server.load("card-counter-red-10.png"),
    ];

    // change projection so that when the window is resized, the game will scale with it while
    // keeping the aspect ratio
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: RENDER_SIZE.y,
            scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(1.),
            ..default()
        },
        ..default()
    });

    // global background for all app states
    commands.insert_resource(ClearColor(CLEAR_COLOR));
    commands.spawn_bundle(SpriteBundle {
        texture: app_assets.background.clone(),
        transform: Transform::from_xyz(0., 0., 0.),
        ..default()
    });

    // initialization complete, show the start menu
    app_state.overwrite_set(AppState::StartMenu).unwrap();
}
