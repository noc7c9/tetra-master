use crate::{card, hover, AppAssets, AppState, CARD_SIZE, COIN_SIZE, RENDER_HSIZE};
use bevy::{prelude::*, sprite::Anchor};
use tetra_master_core as core;

const CARD_EMPHASIZE_OFFSET: Vec3 = Vec3::new(12., 0., 10.);
const CARD_COUNTER_PADDING: Vec2 = Vec2::new(10., 5.);
const COIN_PADDING: Vec2 = Vec2::new(20., 20.);

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup))
            .add_system_set(
                SystemSet::on_update(AppState::InGame).with_system(emphasize_card_on_hover),
                // .with_system(animate_coin)
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

#[derive(Component)]
struct HandCardHoverArea(Entity);

fn setup(
    mut commands: Commands,
    app_assets: Res<AppAssets>,
    player_hands: Query<(Entity, &card::Owner, &Transform), With<card::Card>>,
) {
    // board background
    commands.spawn_bundle(SpriteBundle {
        texture: app_assets.board.clone(),
        transform: Transform::from_xyz(0., 0., 0.1),
        ..default()
    });

    // player hands (already exists, created in the previous state)
    // make each card hoverable
    for (entity, owner, transform) in &player_hands {
        commands
            .entity(entity)
            .insert(Cleanup)
            .insert(hover::Area::new(CARD_SIZE))
            // .insert(crate::debug::rect(CARD_SIZE))
            .insert(HandCardHoverArea(entity));
        // create a sibling hover area prevent rapid hover start/end events
        commands
            .spawn()
            .insert_bundle(TransformBundle::from_transform(*transform))
            .insert(owner.clone())
            .insert(hover::Area::new(CARD_SIZE))
            // .insert(crate::debug::rect(CARD_SIZE))
            .insert(HandCardHoverArea(entity));
    }

    // card counter
    let x = -RENDER_HSIZE.x + CARD_COUNTER_PADDING.x;
    let y = -RENDER_HSIZE.y + CARD_COUNTER_PADDING.y;
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
    let x = RENDER_HSIZE.x - COIN_SIZE.x - COIN_PADDING.x;
    let y = -RENDER_HSIZE.y + COIN_PADDING.y;
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

fn emphasize_card_on_hover(
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    hover_areas: Query<&HandCardHoverArea>,
    mut hand_cards: Query<(&card::Owner, &mut Transform)>,
) {
    for evt in hover_end.iter() {
        let &HandCardHoverArea(entity) = hover_areas.get(evt.entity).unwrap();
        let (owner, mut transform) = hand_cards.get_mut(entity).unwrap();
        transform.translation.x += match owner.0 {
            None => unreachable!(),
            Some(core::Player::P1) => CARD_EMPHASIZE_OFFSET.x,
            Some(core::Player::P2) => -CARD_EMPHASIZE_OFFSET.x,
        };
        transform.translation.z -= CARD_EMPHASIZE_OFFSET.z;
    }
    for evt in hover_start.iter() {
        let &HandCardHoverArea(entity) = hover_areas.get(evt.entity).unwrap();
        let (owner, mut transform) = hand_cards.get_mut(entity).unwrap();
        transform.translation.x += match owner.0 {
            None => unreachable!(),
            Some(core::Player::P1) => -CARD_EMPHASIZE_OFFSET.x,
            Some(core::Player::P2) => CARD_EMPHASIZE_OFFSET.x,
        };
        transform.translation.z += CARD_EMPHASIZE_OFFSET.z;
    }
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
