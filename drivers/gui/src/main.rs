use bevy::{prelude::*, sprite::Anchor};

use tetra_master_core as core;

const RENDER_SIZE: (f32, f32) = (320., 240.);
const SCREEN_SIZE: (f32, f32) = (RENDER_SIZE.0 * 4., RENDER_SIZE.1 * 4.);

// color picked from the background.png file
const CLEAR_COLOR: Color = Color::rgb(0.03137255, 0.03137255, 0.03137255);

fn main() {
    App::new()
        .insert_resource(bevy::render::texture::ImageSettings::default_nearest())
        .insert_resource(WindowDescriptor {
            title: "Tetra Master".to_string(),
            width: SCREEN_SIZE.0,
            height: SCREEN_SIZE.1,
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: RENDER_SIZE.0,
                min_height: RENDER_SIZE.1,
                ..default()
            },
            ..default()
        })
        .insert_resource(GameAssets::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

#[derive(Default)]
struct GameAssets {
    font: Handle<Font>,
    background: Handle<Image>,
    card_blue_bg: Handle<Image>,
    card_red_bg: Handle<Image>,
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
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut assets: ResMut<GameAssets>,
) {
    assets.font = asset_server.load("alexandria.ttf");

    assets.background = asset_server.load("background.png");

    assets.card_blue_bg = asset_server.load("card-bg-blue.png");
    assets.card_red_bg = asset_server.load("card-bg-red.png");

    assets.card_arrow_up = asset_server.load("card-arrow-up.png");
    assets.card_arrow_up_right = asset_server.load("card-arrow-up-right.png");
    assets.card_arrow_right = asset_server.load("card-arrow-right.png");
    assets.card_arrow_down_right = asset_server.load("card-arrow-down-right.png");
    assets.card_arrow_down = asset_server.load("card-arrow-down.png");
    assets.card_arrow_down_left = asset_server.load("card-arrow-down-left.png");
    assets.card_arrow_left = asset_server.load("card-arrow-left.png");
    assets.card_arrow_up_left = asset_server.load("card-arrow-up-left.png");

    assets.card_faces = {
        let handle = asset_server.load("card-faces.png");
        let atlas = TextureAtlas::from_grid(handle, (42., 51.).into(), 10, 10);
        texture_atlases.add(atlas)
    };

    assets.card_stat_font = {
        let handle = asset_server.load("card-stat-font.png");
        let atlas = TextureAtlas::from_grid(handle, (7., 7.).into(), 19, 1);
        texture_atlases.add(atlas)
    };

    commands.insert_resource(ClearColor(CLEAR_COLOR));

    // change projection so that when the window is resized, the game will scale with it while
    // keeping the aspect ratio
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: RENDER_SIZE.1,
            scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(1.),
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(SpriteBundle {
        texture: assets.background.clone(),
        ..default()
    });

    let card = core::Card::physical(1, 2, 3, core::Arrows(10));
    let owner = core::Player::P1;
    let position = (0., 0., 1.).into();
    spawn_card(&mut commands, &assets, position, 0, owner, card);

    let card = core::Card::assault(13, 11, 14, core::Arrows(123));
    let owner = core::Player::P2;
    let position = (25., 10., 2.).into();
    spawn_card(&mut commands, &assets, position, 1, owner, card);
}

fn spawn_card(
    commands: &mut Commands,
    assets: &GameAssets,
    translation: Vec3,
    image: usize,
    owner: core::Player,
    card: core::Card,
) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: match owner {
                core::Player::P1 => assets.card_blue_bg.clone(),
                core::Player::P2 => assets.card_red_bg.clone(),
            },
            transform: Transform::from_translation(translation),
            ..default()
        })
        .with_children(|p| {
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: image,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: assets.card_faces.clone(),

                ..default()
            });

            let x = 9.0;
            let y = 6.0;
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: card.attack as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x, y, 1.),
                ..default()
            });
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: match card.card_type {
                        core::CardType::Physical => 16,
                        core::CardType::Magical => 17,
                        core::CardType::Exploit => 18,
                        core::CardType::Assault => 10,
                    },
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 6., y, 1.),
                ..default()
            });
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: card.physical_defense as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 12., y, 1.),
                ..default()
            });
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: card.magical_defense as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 18., y, 1.),
                ..default()
            });

            for (arrow, texture) in &[
                (core::Arrows::UP, assets.card_arrow_up.clone()),
                (core::Arrows::UP_RIGHT, assets.card_arrow_up_right.clone()),
                (core::Arrows::RIGHT, assets.card_arrow_right.clone()),
                (
                    core::Arrows::DOWN_RIGHT,
                    assets.card_arrow_down_right.clone(),
                ),
                (core::Arrows::DOWN, assets.card_arrow_down.clone()),
                (core::Arrows::DOWN_LEFT, assets.card_arrow_down_left.clone()),
                (core::Arrows::LEFT, assets.card_arrow_left.clone()),
                (core::Arrows::UP_LEFT, assets.card_arrow_up_left.clone()),
            ] {
                if card.arrows.has(*arrow) {
                    p.spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::BottomLeft,
                            ..default()
                        },
                        texture: texture.clone(),
                        transform: Transform::from_xyz(0., 0., 1.),
                        ..default()
                    });
                }
            }
        });
}
