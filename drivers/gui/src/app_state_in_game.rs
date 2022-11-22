use crate::{
    common::{
        calc_board_card_screen_pos, calc_board_cell_screen_pos, calc_hand_card_active_screen_pos,
        calc_hand_card_hovered_screen_pos, calc_hand_card_screen_pos, hand_to_core_hand,
        start_new_game, z_index, BlockedCells, Card, Driver, HandBlue, HandIdx, HandRed, Owner,
        Turn, CELL_SIZE,
    },
    hover, AppAssets, AppState, CARD_SIZE, COIN_SIZE, RENDER_HSIZE,
};
use bevy::{prelude::*, sprite::Anchor};
use rand::prelude::*;
use tetra_master_ai::{self as ai, hybrid_1_simplify::Ai, Ai as _};
use tetra_master_core as core;

const CARD_COUNTER_PADDING: Vec2 = Vec2::new(10., 5.);
const COIN_PADDING: Vec2 = Vec2::new(20., 20.);

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<core::Event>()
            .add_system_set(SystemSet::on_enter(AppState::InGame).with_system(on_enter))
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(
                        place_card
                            // place_card needs to run before so that the active card won't be
                            // dismissed when clicking the board
                            .before(select_and_deselect_card)
                            // since place card generates the events, it needs to run after all the
                            // event handlers
                            // usually it should run before so that events can be handled on the
                            // same frame BUT we need commands to run and we can't use stages due to
                            // limitations in Bevy's State system
                            .after(handle_next_turn_event)
                            .after(handle_flip_event)
                            .after(handle_battle_event)
                            .after(handle_game_over_event)
                            .after(update_card_counter),
                    )
                    .with_system(
                        pick_battle
                            // FIXME: same reasoning as for place_card but this shouldn't be duplicated
                            .after(handle_next_turn_event)
                            .after(handle_flip_event)
                            .after(handle_battle_event)
                            .after(handle_game_over_event)
                            .after(update_card_counter),
                    )
                    .with_system(
                        ai_turn
                            // FIXME: same reasoning as for place_card but this shouldn't be duplicated
                            .after(handle_next_turn_event)
                            .after(handle_flip_event)
                            .after(handle_battle_event)
                            .after(handle_game_over_event)
                            .after(update_card_counter),
                    )
                    .with_system(select_and_deselect_card)
                    .with_system(restart_game)
                    .with_system(maintain_card_hover_marker)
                    .with_system(maintain_cell_hover_marker)
                    .with_system(
                        // TODO: Running this *before* everything else introduces a 1-frame delay.
                        // The properly solution is to run this *after* the Update stage but this
                        // isn't possible when using Bevy states.
                        // Switch to iyes_loopless instead
                        update_card_positions
                            .before(place_card)
                            .before(select_and_deselect_card)
                            .before(maintain_card_hover_marker)
                            .before(maintain_cell_hover_marker),
                    )
                    .with_system(handle_next_turn_event)
                    .with_system(handle_flip_event)
                    .with_system(handle_battle_event)
                    .with_system(handle_game_over_event)
                    .with_system(update_card_counter),
                // .with_system(animate_coin)
            )
            .add_system_set(
                SystemSet::on_exit(AppState::InGame)
                    .with_system(on_exit)
                    .with_system(crate::cleanup::<Cleanup>),
            );
    }
}

#[derive(Debug, Resource)]
enum Status {
    Normal,
    PickingBattle { choices: core::BoardCells },
    GameOver,
}

#[derive(Debug, Resource)]
struct HoveredCard(Option<Entity>);

#[derive(Debug, Resource)]
struct ActiveCard(Option<Entity>);

#[derive(Debug, Resource)]
struct HoveredCell(Option<usize>);

#[derive(Resource)]
struct AI(Ai);

#[derive(Component)]
struct Cleanup;

#[derive(Component)]
struct Coin;

#[derive(Component)]
struct AnimationTimer(Timer);

#[derive(Component)]
struct BlueCardCounter(usize);

#[derive(Component)]
struct RedCardCounter(usize);

#[derive(Component)]
struct HandCardHoverArea(Entity);

#[derive(Component)]
struct BoardCell(usize);

#[derive(Debug, Component)]
struct PlacedCard(usize);

#[derive(Component)]
struct BattlerStatDisplay;

#[derive(Component)]
struct SelectIndicator;

fn on_enter(
    mut commands: Commands,
    app_assets: Res<AppAssets>,
    blocked_cells: Res<BlockedCells>,
    hand_blue: Res<HandBlue>,
    hand_red: Res<HandRed>,
    turn: Res<Turn>,
    player_hands: Query<(Entity, &Owner, &Transform), With<Card>>,
) {
    commands.insert_resource(Status::Normal);
    commands.insert_resource(HoveredCard(None));
    commands.insert_resource(ActiveCard(None));
    commands.insert_resource(HoveredCell(None));

    // board background
    commands
        .spawn(SpriteBundle {
            texture: app_assets.board.clone(),
            transform: Transform::from_xyz(0., 0., z_index::BG + 0.1),
            ..default()
        })
        .insert(Cleanup);

    // blocked cells
    let mut rng = rand::thread_rng();
    for cell in blocked_cells.0 {
        let texture_idx = rng.gen_range(0..app_assets.blocked_cell.len());
        let transform = Transform::from_translation(
            calc_board_cell_screen_pos(cell as usize).extend(z_index::BOARD_BLOCKED_CELL),
        );
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture: app_assets.blocked_cell[texture_idx].clone(),
                transform,
                ..default()
            })
            .insert(Cleanup);
    }

    // board cell hover areas
    for cell in 0usize..16 {
        if blocked_cells.0.has(cell as u8) {
            continue;
        }

        let position = calc_board_cell_screen_pos(cell).extend(z_index::BOARD_CELL_HOVER_AREA);
        let transform = Transform::from_translation(position);
        commands.spawn((
            Cleanup,
            TransformBundle::from_transform(transform),
            hover::Area::new(CELL_SIZE),
            // crate::debug::rect(CELL_SIZE),
            BoardCell(cell),
        ));
    }

    // player hands (already exists, created in the previous state)
    // make each card hoverable
    for (entity, owner, transform) in &player_hands {
        commands
            .entity(entity)
            .insert(Cleanup)
            .insert(hover::Area::new(CARD_SIZE))
            // .insert(crate::debug::rect(CARD_SIZE))
            .insert(HandCardHoverArea(entity));
        // create a sibling hover area to prevent repeated hover start/end events
        commands.spawn((
            TransformBundle::from_transform(*transform),
            owner.clone(),
            hover::Area::new(CARD_SIZE),
            // crate::debug::rect(CARD_SIZE),
            HandCardHoverArea(entity),
        ));
    }

    // card counter
    let x = -RENDER_HSIZE.x + CARD_COUNTER_PADDING.x;
    let y = -RENDER_HSIZE.y + CARD_COUNTER_PADDING.y;
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_center.clone(),
            transform: Transform::from_xyz(x, y, z_index::CARD_COUNTER),
            ..default()
        })
        .insert(Cleanup);
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_red[0].clone(),
            transform: Transform::from_xyz(x, y, z_index::CARD_COUNTER),
            ..default()
        })
        .insert(RedCardCounter(0))
        .insert(Cleanup);
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture: app_assets.card_counter_blue[0].clone(),
            transform: Transform::from_xyz(x, y, z_index::CARD_COUNTER),
            ..default()
        })
        .insert(BlueCardCounter(0))
        .insert(Cleanup);

    // turn indicator coin
    let x = RENDER_HSIZE.x - COIN_SIZE.x - COIN_PADDING.x;
    let y = -RENDER_HSIZE.y + COIN_PADDING.y;
    commands
        .spawn(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: 0,
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture_atlas: app_assets.coin_flip.clone(),
            transform: Transform::from_xyz(x, y, z_index::TURN_INDICATOR_COIN),
            ..default()
        })
        .insert(Coin)
        .insert(AnimationTimer(Timer::from_seconds(
            0.05,
            TimerMode::Repeating,
        )))
        .insert(Cleanup);

    // Setup the AI
    let ai = Ai::init(
        core::Player::Red,
        &core::Setup {
            blocked_cells: blocked_cells.0,
            hand_blue: hand_to_core_hand(&hand_blue.0),
            hand_red: hand_to_core_hand(&hand_red.0),
            battle_system: core::BattleSystem::Deterministic,
            starting_player: turn.0,
        },
    );
    commands.insert_resource(AI(ai));
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<Status>();
    commands.remove_resource::<HoveredCard>();
    commands.remove_resource::<ActiveCard>();
    commands.remove_resource::<HoveredCell>();
}

fn select_and_deselect_card(
    status: Res<Status>,
    turn: Res<Turn>,
    mut active_card: ResMut<ActiveCard>,
    hovered_card: Res<HoveredCard>,
    btns: Res<Input<MouseButton>>,
    owner: Query<&Owner>,
) {
    if !matches!(*status, Status::Normal) {
        return;
    }

    if btns.just_pressed(MouseButton::Left) {
        let owner = hovered_card.0.map(|entity| owner.get(entity).unwrap().0);

        // clicked card belongs to the player whose turn it is
        if owner == Some(turn.0) {
            active_card.0 = if active_card.0 == hovered_card.0 {
                // clicking the active card, deactivates it
                None
            } else {
                // otherwise activate the card
                hovered_card.0
            };
        } else {
            // clicked something else, deactivate active card
            active_card.0 = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pick_battle(
    mut commands: Commands,
    mut event: EventWriter<core::Event>,
    mut driver: ResMut<Driver>,
    mut ai: ResMut<AI>,
    mut status: ResMut<Status>,
    app_assets: Res<AppAssets>,
    hovered_cell: Res<HoveredCell>,
    btns: Res<Input<MouseButton>>,
    select_indicators: Query<Entity, With<SelectIndicator>>,
) {
    let choices = match &*status {
        Status::PickingBattle { choices } => choices,
        _ => return,
    };

    if btns.just_pressed(MouseButton::Left) {
        if let Some(cell) = hovered_cell.0 {
            if !choices.has(cell as u8) {
                return;
            }

            // remove the select indicators
            for entity in &select_indicators {
                commands.entity(entity).despawn_recursive();
            }

            let cmd = core::PickBattle {
                player: core::Player::Blue,
                cell: cell as u8,
            };
            let response = driver.0.send(cmd).expect("PickBattle command should work");

            ai.0.apply_pick_battle(cmd);

            *status = handle_play_ok(
                response,
                &mut commands,
                &mut event,
                &mut driver.0,
                &mut ai.0,
                &app_assets,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn place_card(
    mut commands: Commands,
    mut event: EventWriter<core::Event>,
    mut driver: ResMut<Driver>,
    mut ai: ResMut<AI>,
    mut status: ResMut<Status>,
    mut active_card: ResMut<ActiveCard>,
    mut hovered_cell: ResMut<HoveredCell>,
    app_assets: Res<AppAssets>,
    btns: Res<Input<MouseButton>>,
    hand_idx: Query<&HandIdx>,
    hand_hover_areas: Query<(Entity, &HandCardHoverArea)>,
    board_hover_areas: Query<(Entity, &BoardCell)>,
    transforms: Query<&mut Transform>,
    battler_stat_displays: Query<Entity, With<BattlerStatDisplay>>,
) {
    if !matches!(*status, Status::Normal) {
        return;
    }

    if btns.just_pressed(MouseButton::Left) {
        if let (Some(card_entity), Some(cell)) = (active_card.0, hovered_cell.0) {
            // remove any battlers stats on the screen
            for entity in &battler_stat_displays {
                commands.entity(entity).despawn_recursive();
            }

            let card = hand_idx.get(card_entity).unwrap().0 as u8;

            let cmd = core::PlaceCard {
                player: core::Player::Blue,
                card,
                cell: cell as u8,
            };
            let response = driver.0.send(cmd).expect("PlaceCard command should work");

            ai.0.apply_place_card(cmd);

            place_card_common(
                &mut commands,
                hand_hover_areas,
                board_hover_areas,
                transforms,
                card_entity,
                cell,
            );

            // clear active card
            active_card.0 = None;

            // clear hovered cell
            hovered_cell.0 = None;

            *status = handle_play_ok(
                response,
                &mut commands,
                &mut event,
                &mut driver.0,
                &mut ai.0,
                &app_assets,
            );
        }
    }
}

// common code for placing a card shared between player moves and AI moves
fn place_card_common(
    commands: &mut Commands,
    hand_hover_areas: Query<(Entity, &HandCardHoverArea)>,
    board_hover_areas: Query<(Entity, &BoardCell)>,
    mut transforms: Query<&mut Transform>,
    card_entity: Entity,
    cell: usize,
) {
    // add the PlacedCard marker
    commands.entity(card_entity).insert(PlacedCard(cell));

    // reposition the card
    transforms.get_mut(card_entity).unwrap().translation = calc_board_card_screen_pos(cell);

    // remove the hand hover areas
    commands.entity(card_entity).remove::<HandCardHoverArea>();
    for (area_entity, hover_area) in &hand_hover_areas {
        // remove sibling hover areas
        if area_entity != card_entity && hover_area.0 == card_entity {
            commands.entity(area_entity).despawn_recursive();
        }
    }

    // despawn the board cell hover areas
    for (entity, board_cell) in &board_hover_areas {
        if board_cell.0 == cell {
            commands.entity(entity).despawn_recursive();
            break;
        }
    }
}

fn handle_play_ok(
    play_ok: core::PlayOk,
    commands: &mut Commands,
    event: &mut EventWriter<core::Event>,
    driver: &mut core::Driver,
    ai: &mut impl ai::Ai,
    app_assets: &AppAssets,
) -> Status {
    for evt in play_ok.events {
        event.send(evt)
    }

    if let Some(resolve_battle) = play_ok.resolve_battle {
        let cmd = driver.resolve_battle(resolve_battle);
        ai.apply_resolve_battle(&cmd);
        let response = driver.send(cmd).unwrap();
        return handle_play_ok(response, commands, event, driver, ai, app_assets);
    }

    if play_ok.pick_battle.is_empty() {
        return Status::Normal;
    }

    for cell in play_ok.pick_battle {
        let cell = cell as usize;
        let mut translation = calc_board_card_screen_pos(cell);
        translation.z = z_index::BOARD_CARD_SELECT_INDICATOR;
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture: app_assets.card_select_indicator.clone(),
                transform: Transform::from_translation(translation),
                ..default()
            })
            .insert(hover::Area::new(CELL_SIZE))
            .insert(BoardCell(cell))
            .insert(SelectIndicator);
    }

    Status::PickingBattle {
        choices: play_ok.pick_battle,
    }
}

fn restart_game(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut btns: ResMut<Input<MouseButton>>,
    status: Res<Status>,
    args: Res<crate::Args>,
) {
    if !matches!(*status, Status::GameOver { .. }) {
        return;
    }

    if btns.just_pressed(MouseButton::Left) {
        start_new_game(&mut commands, &mut app_state, &args);

        // required to workaround bug?
        btns.reset(MouseButton::Left);
    }
}

fn maintain_cell_hover_marker(
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    mut hovered_cell: ResMut<HoveredCell>,
    cells: Query<&BoardCell>,
) {
    for evt in hover_end.iter() {
        if let Ok(&BoardCell(cell)) = cells.get(evt.entity) {
            if hovered_cell.0 == Some(cell) {
                hovered_cell.0 = None;
            }
        }
    }
    for evt in hover_start.iter() {
        if let Ok(&BoardCell(cell)) = cells.get(evt.entity) {
            hovered_cell.0 = Some(cell);
        }
    }
}

fn maintain_card_hover_marker(
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    mut hovered_card: ResMut<HoveredCard>,
    hover_areas: Query<&HandCardHoverArea>,
) {
    for evt in hover_end.iter() {
        if let Ok(&HandCardHoverArea(entity)) = hover_areas.get(evt.entity) {
            if hovered_card.0 == Some(entity) {
                hovered_card.0 = None;
            }
        }
    }
    for evt in hover_start.iter() {
        if let Ok(&HandCardHoverArea(entity)) = hover_areas.get(evt.entity) {
            hovered_card.0 = Some(entity);
        }
    }
}

fn update_card_positions(
    hovered_cell: Res<HoveredCell>,
    hovered_card: Res<HoveredCard>,
    active_card: Res<ActiveCard>,
    mut hand_cards: Query<(
        Entity,
        &Owner,
        &HandIdx,
        Option<&PlacedCard>,
        &mut Transform,
    )>,
) {
    if hovered_cell.is_changed() || hovered_card.is_changed() || active_card.is_changed() {
        // iterate over all the cards and set the position for all of them
        for (entity, owner, hand_idx, placed, mut transform) in &mut hand_cards {
            let is_hovered = hovered_card.0 == Some(entity);
            let is_active = active_card.0 == Some(entity);
            let is_over_cell = hovered_cell.0.is_some();
            if let Some(&PlacedCard(cell)) = placed {
                transform.translation = calc_board_card_screen_pos(cell);
            } else if is_hovered {
                transform.translation = calc_hand_card_hovered_screen_pos(owner.0, hand_idx.0);
            } else if is_active && !is_over_cell {
                transform.translation = calc_hand_card_active_screen_pos(owner.0, hand_idx.0);
            } else if is_over_cell && is_active {
                transform.translation = calc_board_card_screen_pos(hovered_cell.0.unwrap());
            } else {
                transform.translation = calc_hand_card_screen_pos(owner.0, hand_idx.0);
            }
        }
    }
}

fn handle_next_turn_event(
    mut event: EventReader<core::Event>,
    mut turn: ResMut<Turn>,
    mut sprite: Query<&mut TextureAtlasSprite, With<Coin>>,
) {
    for event in event.iter() {
        if let core::Event::NextTurn { to } = event {
            turn.0 = *to;
            sprite.single_mut().index = match to {
                core::Player::Blue => 0,
                core::Player::Red => 4,
            };
        }
    }
}

fn handle_flip_event(
    mut event: EventReader<core::Event>,
    app_assets: Res<AppAssets>,
    mut query: Query<(&PlacedCard, &mut Handle<Image>, &mut Owner)>,
) {
    for event in event.iter() {
        if let core::Event::Flip { cell } | core::Event::ComboFlip { cell } = event {
            let mut debug_found_card_in_evt = false;
            for (&PlacedCard(placed_cell), mut image, mut owner) in &mut query {
                if placed_cell == *cell as usize {
                    *image = if *image == app_assets.card_bg_red {
                        owner.0 = core::Player::Blue;
                        app_assets.card_bg_blue.clone()
                    } else {
                        owner.0 = core::Player::Red;
                        app_assets.card_bg_red.clone()
                    };
                    debug_found_card_in_evt = true;
                    break;
                }
            }
            debug_assert!(
                debug_found_card_in_evt,
                "Card in Flip event ({cell}) not a PlacedCard"
            );
        }
    }
}

fn handle_battle_event(
    mut commands: Commands,
    mut event: EventReader<core::Event>,
    app_assets: Res<AppAssets>,
) {
    for event in event.iter() {
        if let core::Event::Battle {
            attacker, defender, ..
        } = event
        {
            spawn_battler_stats(&mut commands, &app_assets, *attacker);
            spawn_battler_stats(&mut commands, &app_assets, *defender);
        }
    }
}

fn handle_game_over_event(
    mut commands: Commands,
    mut event: EventReader<core::Event>,
    mut status: ResMut<Status>,
    app_assets: Res<AppAssets>,
) {
    for event in event.iter() {
        if let core::Event::GameOver { winner } = event {
            let text = match winner {
                None => "It was draw! Left Click to Start a New Game!",
                Some(core::Player::Blue) => "Blue won! Left Click to Start a New Game!",
                Some(core::Player::Red) => "Red won! Left Click to Start a New Game!",
            };
            commands
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .insert(Cleanup)
                .with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                            text,
                            TextStyle {
                                font: app_assets.font.clone(),
                                font_size: 40.0,
                                color: Color::WHITE,
                            },
                        )
                        .with_text_alignment(TextAlignment::CENTER)
                        .with_style(Style {
                            align_self: AlignSelf::FlexStart,
                            position_type: PositionType::Relative,
                            position: UiRect {
                                bottom: Val::Percent(25.0),
                                ..default()
                            },
                            ..default()
                        }),
                    );
                });

            *status = Status::GameOver;
        }
    }
}

fn update_card_counter(
    app_assets: Res<AppAssets>,
    mut red: Query<(&mut RedCardCounter, &mut Handle<Image>), Without<BlueCardCounter>>,
    mut blue: Query<(&mut BlueCardCounter, &mut Handle<Image>), Without<RedCardCounter>>,
    placed_cards: Query<&Owner, With<PlacedCard>>,
) {
    let (mut red_counter, mut red_image) = red.single_mut();
    let (mut blue_counter, mut blue_image) = blue.single_mut();

    let mut red = 0;
    let mut blue = 0;
    for &Owner(owner) in &placed_cards {
        match owner {
            core::Player::Blue => {
                blue += 1;
            }
            core::Player::Red => {
                red += 1;
            }
        }
    }
    red_counter.0 = red;
    blue_counter.0 = blue;
    *red_image = app_assets.card_counter_red[red].clone();
    *blue_image = app_assets.card_counter_blue[blue].clone();
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

fn spawn_battler_stats(commands: &mut Commands, app_assets: &AppAssets, battler: core::Battler) {
    const WIDTH_1_DIGIT_1_POS: Vec2 = Vec2::new(16., 24.);

    const WIDTH_2_DIGIT_1_POS: Vec2 = Vec2::new(11., 24.);
    const WIDTH_2_DIGIT_2_POS: Vec2 = Vec2::new(21., 24.);

    const WIDTH_3_DIGIT_1_POS: Vec2 = Vec2::new(6., 24.);
    const WIDTH_3_DIGIT_2_POS: Vec2 = Vec2::new(16., 24.);
    const WIDTH_3_DIGIT_3_POS: Vec2 = Vec2::new(26., 24.);

    let mut position = calc_board_card_screen_pos(battler.cell as usize);
    position.z = z_index::BOARD_CARD_STARTS;
    let transform = Transform::from_translation(position);
    commands
        .spawn((
            BattlerStatDisplay,
            TransformBundle::from_transform(transform),
            Visibility::default(),
            ComputedVisibility::default(),
            Cleanup,
        ))
        .with_children(|p| {
            fn spawn_digit(
                app_assets: &AppAssets,
                p: &mut ChildBuilder<'_, '_, '_>,
                index: u8,
                position: Vec2,
            ) {
                p.spawn(SpriteSheetBundle {
                    sprite: TextureAtlasSprite {
                        index: index as usize,
                        anchor: Anchor::BottomLeft,
                        ..default()
                    },
                    texture_atlas: app_assets.battle_digits.clone(),
                    transform: Transform::from_translation(position.extend(0.)),
                    ..default()
                });
            }

            // show the stat roll
            match battler.roll {
                0..=9 => {
                    spawn_digit(app_assets, p, battler.roll, WIDTH_1_DIGIT_1_POS);
                }
                10..=99 => {
                    spawn_digit(app_assets, p, battler.roll / 10, WIDTH_2_DIGIT_1_POS);
                    spawn_digit(app_assets, p, battler.roll % 10, WIDTH_2_DIGIT_2_POS);
                }
                _ => {
                    spawn_digit(app_assets, p, battler.roll / 100, WIDTH_3_DIGIT_1_POS);
                    spawn_digit(app_assets, p, battler.roll % 100 / 10, WIDTH_3_DIGIT_2_POS);
                    spawn_digit(app_assets, p, battler.roll % 10, WIDTH_3_DIGIT_3_POS);
                }
            }

            // highlight digit used for the battle
            let x = match battler.digit {
                core::Digit::Attack => 9.,
                core::Digit::PhysicalDefense => 21.,
                core::Digit::MagicalDefense => 27.,
            };
            let y = 6.;
            p.spawn(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    color: Color::GREEN,
                    index: battler.value as usize,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture_atlas: app_assets.card_stat_font.clone(),
                transform: Transform::from_xyz(x, y, 0.),
                ..default()
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn ai_turn(
    mut commands: Commands,
    mut event: EventWriter<core::Event>,
    mut driver: ResMut<Driver>,
    mut ai: ResMut<AI>,
    mut status: ResMut<Status>,
    app_assets: Res<AppAssets>,
    turn: Res<Turn>,
    hand_idx: Query<(Entity, &HandIdx, &Owner)>,
    hand_hover_areas: Query<(Entity, &HandCardHoverArea)>,
    board_hover_areas: Query<(Entity, &BoardCell)>,
    transforms: Query<&mut Transform>,
    battler_stat_displays: Query<Entity, With<BattlerStatDisplay>>,
    select_indicators: Query<Entity, With<SelectIndicator>>,
) {
    // don't play if game is over
    if let Status::GameOver { .. } = *status {
        return;
    }
    // don't play if not AI's turn
    if turn.0 == core::Player::Blue {
        return;
    }

    print!("AI Move");

    #[cfg(not(target_arch = "wasm32"))]
    let now = std::time::Instant::now();

    let ai_cmd = ai.0.get_action();

    #[cfg(not(target_arch = "wasm32"))]
    print!(" | {} ms", now.elapsed().as_millis() as f64 / 1000.);

    println!(" | {ai_cmd:?}");

    // remove any battlers stats on the screen
    for entity in &battler_stat_displays {
        commands.entity(entity).despawn_recursive();
    }

    // remove any select indicators
    for entity in &select_indicators {
        commands.entity(entity).despawn_recursive();
    }

    let response = match ai_cmd {
        ai::Action::PlaceCard(ai_cmd) => {
            let mut card_entity = None;
            for (entity, hand_idx, owner) in &hand_idx {
                if hand_idx.0 as u8 == ai_cmd.card && owner.0 == core::Player::Red {
                    card_entity = Some(entity);
                    break;
                }
            }
            let card_entity = card_entity.unwrap();
            place_card_common(
                &mut commands,
                hand_hover_areas,
                board_hover_areas,
                transforms,
                card_entity,
                ai_cmd.cell as usize,
            );

            driver
                .0
                .send(ai_cmd)
                .expect("AI PlaceCard command should work")
        }
        ai::Action::PickBattle(ai_cmd) => driver
            .0
            .send(ai_cmd)
            .expect("AI PickBattle command should work"),
    };

    ai.0.apply_action(ai_cmd);

    *status = handle_play_ok(
        response,
        &mut commands,
        &mut event,
        &mut driver.0,
        &mut ai.0,
        &app_assets,
    );
}
