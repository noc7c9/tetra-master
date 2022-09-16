use crate::{AppAssets, AppState, RENDER_HSIZE};
use bevy::{prelude::*, sprite::Anchor};

const CARD_COUNTER_PADDING: f32 = 10.;
const COIN_WIDTH: f32 = 40.;
const COIN_PADDING: f32 = 20.;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup))
            .add_system_set(
                SystemSet::on_update(AppState::InGame), /*.with_system(animate_coin)*/
            )
            .add_system_set(
                SystemSet::on_exit(AppState::InGame).with_system(crate::cleanup::<Cleanup>),
            );
    }
}

#[derive(Component)]
struct Cleanup;

#[derive(Component)]
struct Coin;

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Component)]
struct BlueCardCounter(u8);

#[derive(Component)]
struct RedCardCounter(u8);

fn setup(mut commands: Commands, app_assets: Res<AppAssets>) {
    // board background
    commands.spawn_bundle(SpriteBundle {
        texture: app_assets.board.clone(),
        transform: Transform::from_xyz(0., 0., 0.1),
        ..default()
    });

    // card counter
    let x = -RENDER_HSIZE.x + CARD_COUNTER_PADDING;
    let y = -RENDER_HSIZE.y + CARD_COUNTER_PADDING;
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_center.clone(),
            transform: Transform::from_xyz(x, y, 1.0),
            ..default()
        })
        .insert(Cleanup);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_red[0].clone(),
            transform: Transform::from_xyz(x, y, 1.0),
            ..default()
        })
        .insert(RedCardCounter(0))
        .insert(Cleanup);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_blue[0].clone(),
            transform: Transform::from_xyz(x, y, 1.0),
            ..default()
        })
        .insert(BlueCardCounter(0))
        .insert(Cleanup);

    // coin
    let x = RENDER_HSIZE.x - COIN_WIDTH - COIN_PADDING;
    let y = -RENDER_HSIZE.y + COIN_PADDING;
    commands
        .spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: 0,
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture_atlas: app_assets.coin_flip.clone(),
            transform: Transform::from_xyz(x, y, 1.0),
            ..default()
        })
        .insert(Coin)
        .insert(AnimationTimer(Timer::from_seconds(0.05, true)))
        .insert(Cleanup);
}

// fn animate_coin(
//     time: Res<Time>,
//     mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite), With<Coin>>,
// ) {
//     for (mut timer, mut sprite) in &mut query {
//         timer.tick(time.delta());
//         if timer.just_finished() {
//             sprite.index = (sprite.index + 1) % 8;
//         }
//     }
// }
