use super::AppAssets;
use bevy::{prelude::*, sprite::Anchor};
use tetra_master_core as core;

const TOTAL_CARD_IMAGES: usize = 100;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(swap_card_background);

        if cfg!(debug_assertions) {
            app.add_system(dont_allow_card_to_change);
        }
    }
}

#[derive(Component)]
pub struct Card(pub core::Card);

#[derive(Component)]
pub struct Owner(pub Option<core::Player>);

pub(crate) fn spawn<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    app_assets: &AppAssets,
    translation: Vec3,
    owner: Option<core::Player>,
    card: core::Card,
) -> bevy::ecs::system::EntityCommands<'w, 's, 'a> {
    let image_index = card_to_image_index(card);
    let mut entity_commands = commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            anchor: Anchor::BottomLeft,
            ..default()
        },
        texture: match owner {
            None => app_assets.card_bg_gray.clone(),
            Some(core::Player::P1) => app_assets.card_bg_blue.clone(),
            Some(core::Player::P2) => app_assets.card_bg_red.clone(),
        },
        transform: Transform::from_translation(translation),
        ..default()
    });
    entity_commands
        .insert(Card(card))
        .insert(Owner(owner))
        .with_children(|p| {
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: image_index,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: app_assets.card_faces.clone(),
                transform: Transform::from_xyz(0., 0., 0.1),
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
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x, y, 0.2),
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
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 6., y, 0.2),
                ..default()
            });
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: card.physical_defense as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 12., y, 0.2),
                ..default()
            });
            p.spawn_bundle(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: card.magical_defense as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x + 18., y, 0.2),
                ..default()
            });

            for (arrow, texture) in &[
                (core::Arrows::UP, app_assets.card_arrow_up.clone()),
                (
                    core::Arrows::UP_RIGHT,
                    app_assets.card_arrow_up_right.clone(),
                ),
                (core::Arrows::RIGHT, app_assets.card_arrow_right.clone()),
                (
                    core::Arrows::DOWN_RIGHT,
                    app_assets.card_arrow_down_right.clone(),
                ),
                (core::Arrows::DOWN, app_assets.card_arrow_down.clone()),
                (
                    core::Arrows::DOWN_LEFT,
                    app_assets.card_arrow_down_left.clone(),
                ),
                (core::Arrows::LEFT, app_assets.card_arrow_left.clone()),
                (core::Arrows::UP_LEFT, app_assets.card_arrow_up_left.clone()),
            ] {
                if card.arrows.has(*arrow) {
                    p.spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::BottomLeft,
                            ..default()
                        },
                        texture: texture.clone(),
                        transform: Transform::from_xyz(0., 0., 0.2),
                        ..default()
                    });
                }
            }
        });
    entity_commands
}

fn card_to_image_index(card: core::Card) -> usize {
    let mut hash = match card.card_type {
        core::CardType::Physical => 1,
        core::CardType::Magical => 2,
        core::CardType::Exploit => 3,
        core::CardType::Assault => 4,
    };
    hash += 3 * card.attack as usize;
    hash += 5 * card.physical_defense as usize;
    hash += 7 * card.magical_defense as usize;
    hash % TOTAL_CARD_IMAGES
}

fn swap_card_background(
    app_assets: Res<AppAssets>,
    mut query: Query<(&mut Handle<Image>, &Owner), Changed<Owner>>,
) {
    for (mut texture, owner) in &mut query {
        *texture = match owner.0 {
            None => app_assets.card_bg_gray.clone(),
            Some(core::Player::P1) => app_assets.card_bg_blue.clone(),
            Some(core::Player::P2) => app_assets.card_bg_red.clone(),
        };
    }
}

#[cfg(debug_assertions)]
fn dont_allow_card_to_change(query: Query<ChangeTrackers<Card>>) {
    for tracker in &query {
        if tracker.is_changed() && !tracker.is_added() {
            panic!("Card should not change after initial insertion")
        }
    }
}
