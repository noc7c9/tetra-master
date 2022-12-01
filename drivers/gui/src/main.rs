use bevy::prelude::*;
use clap::Parser;

#[macro_use]
mod macros;

mod debug;

mod camera;
mod common;
mod hover;
mod layout;
mod random_setup_generator;

mod app_state_in_game;
mod app_state_picking_hands;
mod app_state_start_menu;

use layout::{TransformExt as _, Z};

// all image assets are created with ASSET_SIZE as the assumed screen size
const ASSET_SIZE: Vec2 = Vec2::new(320., 240.);

// but render assets at a larger size so that text can be rendered at a higher resolution
const ASSET_SCALE: f32 = 4.0;
const RENDER_SIZE: Vec2 = vec2!(ASSET_SIZE * ASSET_SCALE);

const CARD_ASSET_SIZE: Vec2 = Vec2::new(42., 51.);
const COIN_ASSET_SIZE: Vec2 = Vec2::new(40., 40.);
const CURSOR_ASSET_SIZE: Vec2 = Vec2::new(24., 13.);
const DIGIT_ASSET_SIZE: Vec2 = Vec2::new(10., 14.);
const CARD_COUNTER_ASSET_SIZE: Vec2 = Vec2::new(57., 65.);

const CARD_SIZE: Vec2 = vec2!(CARD_ASSET_SIZE * ASSET_SCALE);
const COIN_SIZE: Vec2 = vec2!(COIN_ASSET_SIZE * ASSET_SCALE);
const CURSOR_SIZE: Vec2 = vec2!(CURSOR_ASSET_SIZE * ASSET_SCALE);
const CARD_COUNTER_SIZE: Vec2 = vec2!(CARD_COUNTER_ASSET_SIZE * ASSET_SCALE);

// color picked from the background.png file
const CLEAR_COLOR: Color = Color::rgb(0.03137255, 0.03137255, 0.03137255);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Initialization,
    StartMenu,
    PickingHands,
    InGame,
}

#[derive(Debug, Parser, Resource)]
struct Args {
    #[clap(value_name = "PATH")]
    /// Path to the external implementation to use, if omitted the reference implementation will be
    /// used
    implementation: Option<String>,
}

fn main() {
    App::new()
        .insert_resource(Args::parse())
        .insert_resource(AppAssets::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "Tetra Master".to_string(),
                        width: RENDER_SIZE.x,
                        height: RENDER_SIZE.y,
                        fit_canvas_to_parent: true,
                        ..default()
                    },
                    ..default()
                }),
        )
        .add_plugin(camera::Plugin)
        .add_plugin(debug::Plugin)
        .add_plugin(common::Plugin)
        .add_plugin(hover::Plugin)
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

#[derive(Default, Resource)]
struct AppAssets {
    font: Handle<Font>,
    background: Handle<Image>,
    battle_digits: Handle<TextureAtlas>,
    board: Handle<Image>,
    blocked_cell: [Handle<Image>; 2],
    coin_flip: Handle<TextureAtlas>,
    cursor: Handle<TextureAtlas>,
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
    card_select_indicator: Handle<Image>,
}

fn setup(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut app_assets: ResMut<AppAssets>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    option_env!("GIT_SHA").map(|sha| log::info!("Build: {sha}"));

    // load assets
    app_assets.font = asset_server.load("alexandria.ttf");

    app_assets.background = asset_server.load("background.png");
    app_assets.board = asset_server.load("board.png");

    app_assets.battle_digits = {
        let handle = asset_server.load("battle-digits.png");
        let atlas = TextureAtlas::from_grid(handle, DIGIT_ASSET_SIZE, 10, 1, None, None);
        texture_atlases.add(atlas)
    };

    app_assets.blocked_cell = [
        asset_server.load("blocked-cell-1.png"),
        asset_server.load("blocked-cell-2.png"),
    ];

    app_assets.coin_flip = {
        let handle = asset_server.load("coin-flip.png");
        let atlas = TextureAtlas::from_grid(handle, COIN_ASSET_SIZE, 8, 1, None, None);
        texture_atlases.add(atlas)
    };

    app_assets.cursor = {
        let handle = asset_server.load("cursor.png");
        let atlas = TextureAtlas::from_grid(handle, CURSOR_ASSET_SIZE, 8, 1, None, None);
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
        let atlas = TextureAtlas::from_grid(handle, CARD_ASSET_SIZE, 10, 10, None, None);
        texture_atlases.add(atlas)
    };

    app_assets.card_stat_font = {
        let handle = asset_server.load("card-stat-font.png");
        let atlas = TextureAtlas::from_grid(handle, (7., 7.).into(), 19, 1, None, None);
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

    app_assets.card_select_indicator = asset_server.load("card-select-indicator.png");

    commands.spawn(camera::Camera::new(RENDER_SIZE));

    // global background for all app states
    commands.insert_resource(ClearColor(CLEAR_COLOR));
    commands.spawn(SpriteBundle {
        texture: app_assets.background.clone(),
        transform: layout::center().z(Z::BG),
        ..default()
    });

    // initialization complete, show the start menu
    app_state.overwrite_set(AppState::StartMenu).unwrap();
}
