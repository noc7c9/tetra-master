use crate::{AppAssets, AppState, CARD_SIZE, RENDER_HSIZE};
use bevy::{prelude::*, sprite::Anchor};
use tetra_master_core as core;

const TOTAL_CARD_IMAGES: usize = 100;

const PLAYER_HAND_VOFFSET: f32 = 27.;
const PLAYER_HAND_PADDING: f32 = 4.;

const BOARD_POS: Vec2 = Vec2::new(-88.5, -95.5);
pub const CELL_SIZE: Vec2 = Vec2::new(CARD_SIZE.x + 1., CARD_SIZE.y + 1.);

pub const CANDIDATE_PADDING: f32 = 3.;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        {
            app.add_system(dont_allow_card_to_change);
        }
    }
}

pub struct Driver(pub core::Driver);

#[derive(Debug)]
pub struct Turn(pub core::Player);

#[derive(Debug)]
pub struct Candidates(pub [Option<core::Hand>; 3]);

#[derive(Debug)]
pub struct BlockedCells(pub Vec<usize>);

#[derive(Debug, Component, Clone)]
pub struct Card(pub core::Card);

#[derive(Debug, Component, Clone)]
pub struct Owner(pub core::Player);

#[derive(Debug, Component, Clone)]
pub struct OptionalOwner(pub Option<core::Player>);

#[derive(Debug, Component, Clone)]
pub struct HandIdx(
    /// index into the hand, from top to bottom
    pub usize,
);

pub(crate) fn start_new_game(
    commands: &mut Commands,
    app_state: &mut State<AppState>,
    args: &crate::Args,
) {
    // start the new game
    let mut driver = match &args.implementation {
        Some(implementation) => core::Driver::external(implementation),
        None => core::Driver::reference(),
    }
    .log()
    .build();
    // TODO: handle the error
    let response = driver
        .send_random_setup(core::BattleSystem::Dice { sides: 12 })
        .unwrap();
    let c = response.hand_candidates;
    commands.insert_resource(Candidates([Some(c[0]), Some(c[1]), Some(c[2])]));
    commands.insert_resource(BlockedCells(
        response
            .blocked_cells
            .into_iter()
            .map(|c| c as usize)
            .collect(),
    ));
    commands.insert_resource(Turn(core::Player::P1));

    commands.insert_resource(Driver(driver));

    // change the state
    app_state.set(AppState::PickingHands).unwrap();
}

pub(crate) fn spawn_card<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    app_assets: &AppAssets,
    translation: Vec3,
    card: core::Card,
) -> bevy::ecs::system::EntityCommands<'w, 's, 'a> {
    let image_index = card_to_image_index(card);
    let mut entity_commands = commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            anchor: Anchor::BottomLeft,
            ..default()
        },
        texture: app_assets.card_bg_gray.clone(),
        transform: Transform::from_translation(translation),
        ..default()
    });
    entity_commands.insert(Card(card)).with_children(|p| {
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

pub fn calc_candidate_card_screen_pos(candidate_idx: usize, hand_idx: usize) -> Vec2 {
    let candidate_idx = candidate_idx as f32;
    let hand_idx = hand_idx as f32;
    Vec2::new(
        CARD_SIZE.x * -2.5 + CANDIDATE_PADDING * -2. + hand_idx * (CARD_SIZE.x + CANDIDATE_PADDING),
        CARD_SIZE.y * 0.5 + CANDIDATE_PADDING + candidate_idx * -(CARD_SIZE.y + CANDIDATE_PADDING),
    )
}

pub fn calc_hand_card_screen_pos(owner: core::Player, hand_idx: usize) -> Vec3 {
    let hand_idx = hand_idx as f32;
    Vec3::new(
        match owner {
            core::Player::P1 => RENDER_HSIZE.x - CARD_SIZE.x - PLAYER_HAND_PADDING,
            core::Player::P2 => -RENDER_HSIZE.x + PLAYER_HAND_PADDING,
        },
        RENDER_HSIZE.y - CARD_SIZE.y - PLAYER_HAND_PADDING - PLAYER_HAND_VOFFSET * hand_idx,
        1. + hand_idx,
    )
}

pub fn calc_board_cell_screen_pos(cell: usize) -> Vec2 {
    Vec2::new(
        BOARD_POS.x + (cell % 4) as f32 * CELL_SIZE.x,
        BOARD_POS.y + (3 - cell / 4) as f32 * CELL_SIZE.y,
    )
}

pub fn calc_board_card_screen_pos(cell: usize) -> Vec3 {
    Vec3::new(
        BOARD_POS.x + (cell % 4) as f32 * CELL_SIZE.x + 0.5,
        BOARD_POS.y + (3 - cell / 4) as f32 * CELL_SIZE.y + 0.5,
        2.,
    )
}

#[cfg(debug_assertions)]
fn dont_allow_card_to_change(query: Query<ChangeTrackers<Card>>) {
    for tracker in &query {
        if tracker.is_changed() && !tracker.is_added() {
            panic!("Card should not change after initial insertion")
        }
    }
}
