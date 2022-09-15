use bevy::prelude::*;
use clap::Parser;

mod debug;

mod card;
mod game_state;

mod app_state_picking_hands;
mod app_state_start_menu;

const RENDER_SIZE: Vec2 = Vec2::new(320., 240.);
const RENDER_HSIZE: Vec2 = Vec2::new(RENDER_SIZE.x / 2., RENDER_SIZE.y / 2.);
const SCREEN_SIZE: Vec2 = Vec2::new(RENDER_SIZE.x * 4., RENDER_SIZE.y * 4.);

// color picked from the background.png file
const CLEAR_COLOR: Color = Color::rgb(0.03137255, 0.03137255, 0.03137255);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Initialization,
    StartMenu,
    PickingHands,
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
        let atlas = TextureAtlas::from_grid(handle, (42., 51.).into(), 10, 10);
        texture_atlases.add(atlas)
    };

    app_assets.card_stat_font = {
        let handle = asset_server.load("card-stat-font.png");
        let atlas = TextureAtlas::from_grid(handle, (7., 7.).into(), 19, 1);
        texture_atlases.add(atlas)
    };

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
        ..default()
    });

    // initialization complete, show the start menu
    app_state.overwrite_set(AppState::StartMenu).unwrap();
}
