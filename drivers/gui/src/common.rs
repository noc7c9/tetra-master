use crate::{
    layout::{self, TransformExt as _, Z},
    AppAssets, AppState, CARD_SIZE,
};
use bevy::prelude::*;
use tetra_master_core as core;

const PLAYER_HAND_VOFFSET: f32 = 94.;
const PLAYER_HAND_PADDING: f32 = 16.;

const PLAYER_HAND_ACTIVE_HOFFSET: f32 = 48.;
const PLAYER_HAND_HOVERED_HOFFSET: f32 = 48.;

const BOARD_POS: Vec2 = Vec2::new(-354., -382.);
pub const CELL_SIZE: Vec2 = vec2!(CARD_SIZE + (4., 4.));

pub const CANDIDATE_PADDING: f32 = 12.;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    #[allow(unused_variables)]
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        {
            app.add_system(dont_allow_card_to_change);
        }
    }
}

#[derive(Resource)]
pub struct Driver(pub core::Driver);

#[derive(Debug, Resource)]
pub struct Turn(pub core::Player);

#[derive(Debug, Resource)]
pub struct BattleSystem(pub core::BattleSystem);

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

#[derive(Debug, Resource)]
pub struct HandRed(pub Hand);

#[derive(Debug, Resource)]
pub struct HandBlue(pub Hand);

#[derive(Debug, Resource)]
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
    commands.insert_resource(BattleSystem(response.battle_system));
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
    transform: Transform,
    card: Card,
    owner: Option<core::Player>,
) -> bevy::ecs::system::EntityCommands<'w, 's, 'a> {
    let mut entity_commands = commands.spawn(SpriteBundle {
        texture: match owner {
            None => app_assets.card_bg_gray.clone(),
            Some(core::Player::Blue) => app_assets.card_bg_blue.clone(),
            Some(core::Player::Red) => app_assets.card_bg_red.clone(),
        },
        transform,
        ..default()
    });

    entity_commands.insert(card).with_children(|p| {
        p.spawn(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: card.image_index,
                ..default()
            },
            texture_atlas: app_assets.card_faces.clone(),
            transform: Transform::from_xyz(0., 0., 0.1),
            ..default()
        });

        for (arrow, texture) in [
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
            if card.stats.arrows.has_any(arrow) {
                p.spawn(SpriteBundle {
                    texture: texture.clone(),
                    transform: Transform::from_xyz(0., 0., 0.2),
                    ..default()
                });
            }
        }

        for (index, x) in [
            (card.stats.attack as usize, -8.5),
            (
                match card.stats.card_type {
                    core::CardType::Physical => 16,
                    core::CardType::Magical => 17,
                    core::CardType::Exploit => 18,
                    core::CardType::Assault => 10,
                },
                -2.5,
            ),
            (card.stats.physical_defense as usize, 3.5),
            (card.stats.magical_defense as usize, 9.5),
        ] {
            p.spawn(SpriteSheetBundle {
                sprite: TextureAtlasSprite { index, ..default() },
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x, -16., 0.2),
                ..default()
            });
        }
    });

    entity_commands
}

pub fn calc_candidate_card_screen_pos(candidate_idx: usize, hand_idx: usize) -> Transform {
    // calc center position of the layout
    let pos = layout::center()
        .z(Z::CANDIDATE_HAND_CARD)
        // offset based on whether this is the blue or red hand
        .offset_y(-(candidate_idx as f32 - 0.5) * (CARD_SIZE.y + CANDIDATE_PADDING));

    layout::line_horizontal(pos)
        .num_entities(core::HAND_SIZE)
        .entity_size(CARD_SIZE)
        .padding(CANDIDATE_PADDING)
        .index(hand_idx)
        .offset_z(hand_idx as f32)
}

pub fn calc_hand_card_screen_pos(owner: core::Player, hand_idx: usize) -> Transform {
    // size is smaller, making the cards overlap
    let size = CARD_SIZE - Vec2::new(0., PLAYER_HAND_VOFFSET);

    let offset = CARD_SIZE.x / 2. + PLAYER_HAND_PADDING;
    let pos = match owner {
        core::Player::Blue => layout::top_right().offset_x(-offset),
        core::Player::Red => layout::top_left().offset_x(offset),
    }
    .offset_y(-size.y * 1.5 - CARD_SIZE.y + PLAYER_HAND_PADDING * 2.)
    .z(Z::HAND_CARD);

    layout::line_vertical(pos)
        .num_entities(core::HAND_SIZE)
        .entity_size(size)
        .index(hand_idx)
        .offset_z(hand_idx as f32)
}

pub fn calc_hand_card_active_screen_pos(owner: core::Player, hand_idx: usize) -> Transform {
    calc_hand_card_screen_pos(owner, hand_idx)
        .offset_x(match owner {
            core::Player::Blue => -PLAYER_HAND_ACTIVE_HOFFSET,
            core::Player::Red => PLAYER_HAND_ACTIVE_HOFFSET,
        })
        .offset_z(Z::HAND_CARD_ACTIVE)
}

pub fn calc_hand_card_hovered_screen_pos(owner: core::Player, hand_idx: usize) -> Transform {
    calc_hand_card_screen_pos(owner, hand_idx)
        .offset_x(match owner {
            core::Player::Blue => -PLAYER_HAND_HOVERED_HOFFSET,
            core::Player::Red => PLAYER_HAND_HOVERED_HOFFSET,
        })
        .offset_z(Z::HAND_CARD_HOVERED)
}

pub fn calc_blocked_cell_screen_pos(cell: usize) -> Transform {
    layout::absolute(BOARD_POS)
        .z(Z::BOARD_BLOCKED_CELL)
        .offset(CARD_SIZE / 2.)
        .offset((
            (cell % 4) as f32 * CELL_SIZE.x + 2.,
            (3 - cell / 4) as f32 * CELL_SIZE.y + 2.,
        ))
}

pub fn calc_board_cell_hover_area_screen_pos(cell: usize) -> Transform {
    layout::absolute(BOARD_POS)
        .z(Z::BOARD_CELL_HOVER_AREA)
        .scale(1.)
        .offset(CARD_SIZE / 2.)
        .offset((
            (cell % 4) as f32 * CELL_SIZE.x,
            (3 - cell / 4) as f32 * CELL_SIZE.y,
        ))
}

pub fn calc_board_card_screen_pos(cell: usize) -> Transform {
    layout::absolute(BOARD_POS)
        .z(Z::BOARD_CARD)
        .offset(CARD_SIZE / 2.)
        .offset((
            (cell % 4) as f32 * CELL_SIZE.x + 2.,
            (3 - cell / 4) as f32 * CELL_SIZE.y + 2.,
        ))
}

#[cfg(debug_assertions)]
fn dont_allow_card_to_change(query: Query<ChangeTrackers<Card>>) {
    for tracker in &query {
        if tracker.is_changed() && !tracker.is_added() {
            panic!("Card should not change after initial insertion")
        }
    }
}
