use crate::{AppAssets, AppState, CARD_SIZE, RENDER_HSIZE};
use bevy::{prelude::*, sprite::Anchor};
use tetra_master_core as core;

const PLAYER_HAND_VOFFSET: f32 = 27.;
const PLAYER_HAND_PADDING: f32 = 4.;

const PLAYER_HAND_ACTIVE_HOFFSET: f32 = 12.;
const PLAYER_HAND_HOVERED_HOFFSET: f32 = 12.;

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

pub mod z_index {
    pub const BG: f32 = 0.;

    pub const CARD_COUNTER: f32 = 1.;
    pub const TURN_INDICATOR_COIN: f32 = 1.;

    pub const CANDIDATE_HAND_CARD: f32 = 1.;

    pub const HAND_CARD: f32 = 1.;
    pub const HAND_CARD_ACTIVE: f32 = 5.;
    pub const HAND_CARD_HOVERED: f32 = 10.;

    pub const BOARD_CARD: f32 = 1.;
    pub const BOARD_BLOCKED_CELL: f32 = 1.;
    pub const BOARD_CARD_STARTS: f32 = 2.;
    pub const BOARD_CARD_SELECT_INDICATOR: f32 = 2.;

    // hover areas
    // pub const CANDIDATE_HAND_HOVER_AREA: f32 = 100.;
    pub const BOARD_CELL_HOVER_AREA: f32 = 100.;

    pub const DEBUG: f32 = 666.;
}

pub struct Driver(pub core::Driver);

#[derive(Debug)]
pub struct Turn(pub core::Player);

// #[derive(Debug)]
// pub struct Candidates(pub [core::Hand; 3]);

pub type Hand = [Card; core::HAND_SIZE];

pub fn hand_to_core_hand(hand: &Hand) -> core::Hand {
    [
        hand[0].stats,
        hand[1].stats,
        hand[2].stats,
        hand[3].stats,
        hand[4].stats,
    ]
}

#[derive(Debug)]
pub struct HandRed(pub Hand);

#[derive(Debug)]
pub struct HandBlue(pub Hand);

#[derive(Debug)]
pub struct BlockedCells(pub core::BoardCells);

#[derive(Debug, Component, Clone, Copy)]
pub struct Card {
    pub image_index: usize,
    pub name: &'static str,
    pub stats: core::Card,
}

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
    // .seed(15256155310125961462)
    .build();

    let mut rng = driver.get_rng();
    let starting_player = crate::random_setup_generator::random_starting_player(&mut rng);
    let blocked_cells = crate::random_setup_generator::random_blocked_cells(&mut rng);
    let hand_blue = crate::random_setup_generator::random_hand(&mut rng);
    let hand_red = crate::random_setup_generator::random_hand(&mut rng);
    let setup = core::Setup {
        battle_system: core::BattleSystem::Dice { sides: 12 },
        starting_player,
        blocked_cells,
        hand_blue: crate::common::hand_to_core_hand(&hand_blue),
        hand_red: crate::common::hand_to_core_hand(&hand_red),
    };

    // TODO: handle the error
    let response = driver.send(setup).unwrap();

    // commands.insert_resource(Candidates(response.hand_candidates));
    commands.insert_resource(HandBlue(hand_blue));
    commands.insert_resource(HandRed(hand_red));
    commands.insert_resource(BlockedCells(response.blocked_cells));
    commands.insert_resource(Turn(response.starting_player));

    commands.insert_resource(Driver(driver));

    // change the state
    app_state.set(AppState::PickingHands).unwrap();
}

pub(crate) fn spawn_card<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    app_assets: &AppAssets,
    translation: Vec3,
    card: Card,
    owner: Option<core::Player>,
) -> bevy::ecs::system::EntityCommands<'w, 's, 'a> {
    let mut entity_commands = commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            anchor: Anchor::BottomLeft,
            ..default()
        },
        texture: match owner {
            None => app_assets.card_bg_gray.clone(),
            Some(core::Player::Blue) => app_assets.card_bg_blue.clone(),
            Some(core::Player::Red) => app_assets.card_bg_red.clone(),
        },
        transform: Transform::from_translation(translation),
        ..default()
    });
    entity_commands.insert(card).with_children(|p| {
        p.spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: card.image_index,
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
                index: card.stats.attack as usize,
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture_atlas: app_assets.card_stat_font.clone(),
            transform: Transform::from_xyz(x, y, 0.2),
            ..default()
        });
        p.spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: match card.stats.card_type {
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
                index: card.stats.physical_defense as usize,
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture_atlas: app_assets.card_stat_font.clone(),
            transform: Transform::from_xyz(x + 12., y, 0.2),
            ..default()
        });
        p.spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: card.stats.magical_defense as usize,
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
            if card.stats.arrows.has_any(*arrow) {
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

pub fn calc_candidate_card_screen_pos(candidate_idx: usize, hand_idx: usize) -> Vec3 {
    let candidate_idx = candidate_idx as f32;
    let hand_idx = hand_idx as f32;
    Vec3::new(
        CARD_SIZE.x * -2.5 + CANDIDATE_PADDING * -2. + hand_idx * (CARD_SIZE.x + CANDIDATE_PADDING),
        CARD_SIZE.y * 0.5 + CANDIDATE_PADDING + candidate_idx * -(CARD_SIZE.y + CANDIDATE_PADDING),
        z_index::CANDIDATE_HAND_CARD,
    )
}

pub fn calc_hand_card_screen_pos(owner: core::Player, hand_idx: usize) -> Vec3 {
    let hand_idx = hand_idx as f32;
    Vec3::new(
        match owner {
            core::Player::Blue => RENDER_HSIZE.x - CARD_SIZE.x - PLAYER_HAND_PADDING,
            core::Player::Red => -RENDER_HSIZE.x + PLAYER_HAND_PADDING,
        },
        RENDER_HSIZE.y - CARD_SIZE.y - PLAYER_HAND_PADDING - PLAYER_HAND_VOFFSET * hand_idx,
        z_index::HAND_CARD + core::HAND_SIZE as f32 - hand_idx,
    )
}

pub fn calc_hand_card_active_screen_pos(owner: core::Player, hand_idx: usize) -> Vec3 {
    let mut pos = calc_hand_card_screen_pos(owner, hand_idx);
    pos.x += match owner {
        core::Player::Blue => -PLAYER_HAND_ACTIVE_HOFFSET,
        core::Player::Red => PLAYER_HAND_ACTIVE_HOFFSET,
    };
    pos.z += z_index::HAND_CARD_ACTIVE;
    pos
}

pub fn calc_hand_card_hovered_screen_pos(owner: core::Player, hand_idx: usize) -> Vec3 {
    let mut pos = calc_hand_card_screen_pos(owner, hand_idx);
    pos.x += match owner {
        core::Player::Blue => -PLAYER_HAND_HOVERED_HOFFSET,
        core::Player::Red => PLAYER_HAND_HOVERED_HOFFSET,
    };
    pos.z += z_index::HAND_CARD_HOVERED;
    pos
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
        z_index::BOARD_CARD,
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
