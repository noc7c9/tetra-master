use crate::{
    common::{
        calc_board_card_screen_pos, calc_board_cell_screen_pos, calc_hand_card_screen_pos,
        BlockedCells, Card, Driver, HandIdx, Owner, Turn, CELL_SIZE,
    },
    hover, AppAssets, AppState, CARD_SIZE, COIN_SIZE, RENDER_HSIZE,
};
use bevy::{prelude::*, sprite::Anchor};
use tetra_master_core as core;

const CARD_EMPHASIZE_OFFSET: Vec3 = Vec3::new(12., 0., 5.);
const CARD_COUNTER_PADDING: Vec2 = Vec2::new(10., 5.);
const COIN_PADDING: Vec2 = Vec2::new(20., 20.);

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NextTurn>()
            .add_event::<Flip>()
            .add_event::<Battle>()
            .add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup))
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
                            .after(handle_next_turn)
                            .after(handle_flip)
                            .after(handle_battle)
                            .after(update_card_counter),
                    )
                    .with_system(select_and_deselect_card)
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
                    .with_system(handle_next_turn)
                    .with_system(handle_flip)
                    .with_system(handle_battle)
                    .with_system(update_card_counter),
                // .with_system(animate_coin)
            )
            .add_system_set(
                SystemSet::on_exit(AppState::InGame).with_system(crate::cleanup::<Cleanup>),
            );
    }
}

struct NextTurn {
    to: core::Player,
}

struct Flip {
    cell: u8,
}

#[derive(Debug)]
struct Battle {
    attacker: core::Battler,
    defender: core::Battler,
}

#[derive(Debug)]
struct HoveredCard(Option<Entity>);

#[derive(Debug)]
struct ActiveCard(Option<Entity>);

#[derive(Debug)]
struct HoveredCell(Option<usize>);

#[derive(Component)]
struct Cleanup;

#[derive(Component)]
struct Coin;

#[derive(Component, Deref, DerefMut)]
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

fn setup(
    mut commands: Commands,
    app_assets: Res<AppAssets>,
    blocked_cells: Res<BlockedCells>,
    player_hands: Query<(Entity, &Owner, &Transform), With<Card>>,
) {
    commands.insert_resource(HoveredCard(None));
    commands.insert_resource(ActiveCard(None));
    commands.insert_resource(HoveredCell(None));

    // board background
    commands
        .spawn_bundle(SpriteBundle {
            texture: app_assets.board.clone(),
            transform: Transform::from_xyz(0., 0., 0.1),
            ..default()
        })
        .insert(Cleanup);

    // blocked cells
    for &cell in &blocked_cells.0 {
        let texture_idx = fastrand::usize(..app_assets.blocked_cell.len());
        let transform = Transform::from_translation(calc_board_cell_screen_pos(cell).extend(0.2));
        commands
            .spawn_bundle(SpriteBundle {
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
    for cell in 0..16 {
        if blocked_cells.0.contains(&cell) {
            continue;
        }

        let transform = Transform::from_translation(calc_board_cell_screen_pos(cell).extend(100.));
        commands
            .spawn()
            .insert(Cleanup)
            .insert_bundle(TransformBundle::from_transform(transform))
            .insert(hover::Area::new(CELL_SIZE))
            // .insert(crate::debug::rect(CELL_SIZE))
            .insert(BoardCell(cell));
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

fn select_and_deselect_card(
    turn: Res<Turn>,
    mut active_card: ResMut<ActiveCard>,
    hovered_card: Res<HoveredCard>,
    btns: Res<Input<MouseButton>>,
    owner: Query<&Owner>,
) {
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
fn place_card(
    mut commands: Commands,
    mut next_turn: EventWriter<NextTurn>,
    mut flip: EventWriter<Flip>,
    mut battle: EventWriter<Battle>,
    mut driver: ResMut<Driver>,
    mut active_card: ResMut<ActiveCard>,
    hovered_cell: Res<HoveredCell>,
    btns: Res<Input<MouseButton>>,
    hand_idx: Query<&HandIdx>,
    hand_hover_areas: Query<(Entity, &HandCardHoverArea)>,
    board_hover_areas: Query<(Entity, &BoardCell)>,
    mut transforms: Query<&mut Transform>,
    mut battler_stat_displays: Query<Entity, With<BattlerStatDisplay>>,
) {
    if btns.just_pressed(MouseButton::Left) {
        if let (Some(card_entity), Some(cell)) = (active_card.0, hovered_cell.0) {
            // remove any battlers stats on the screen
            for entity in battler_stat_displays.iter_mut() {
                commands.entity(entity).despawn_recursive();
            }

            let card = hand_idx.get(card_entity).unwrap().0 as u8;

            let response = driver
                .0
                .send(core::command::PlaceCard {
                    card,
                    cell: cell as u8,
                })
                .expect("PlaceCard command should work");

            for event in response.events {
                match event {
                    core::Event::NextTurn { to } => next_turn.send(NextTurn { to }),
                    core::Event::Flip { cell } | core::Event::ComboFlip { cell } => {
                        flip.send(Flip { cell })
                    }
                    core::Event::Battle {
                        attacker, defender, ..
                    } => battle.send(Battle { attacker, defender }),
                    core::Event::GameOver { .. } => todo!("{event:?}"),
                }
            }

            commands.entity(card_entity).insert(PlacedCard(cell));

            // reposition the card
            transforms.get_mut(card_entity).unwrap().translation = calc_board_card_screen_pos(cell);

            // clear active card
            active_card.0 = None;

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
            transform.translation = calc_hand_card_screen_pos(owner.0, hand_idx.0);

            let is_hovered = hovered_card.0 == Some(entity);
            let is_active = active_card.0 == Some(entity);
            let is_over_cell = hovered_cell.0.is_some();
            if let Some(&PlacedCard(cell)) = placed {
                transform.translation = calc_board_card_screen_pos(cell);
            } else if is_hovered || (is_active && !is_over_cell) {
                transform.translation.x += match owner.0 {
                    core::Player::P1 => -CARD_EMPHASIZE_OFFSET.x,
                    core::Player::P2 => CARD_EMPHASIZE_OFFSET.x,
                };
                transform.translation.z += CARD_EMPHASIZE_OFFSET.z;
            } else if is_over_cell && is_active {
                transform.translation = calc_board_card_screen_pos(hovered_cell.0.unwrap());
            }
        }
    }
}

fn handle_next_turn(
    mut next_turn: EventReader<NextTurn>,
    mut turn: ResMut<Turn>,
    mut sprite: Query<&mut TextureAtlasSprite, With<Coin>>,
) {
    for NextTurn { to } in next_turn.iter() {
        turn.0 = *to;
        sprite.single_mut().index = match to {
            core::Player::P1 => 0,
            core::Player::P2 => 4,
        };
    }
}

fn handle_flip(
    mut flip: EventReader<Flip>,
    app_assets: Res<AppAssets>,
    mut query: Query<(&PlacedCard, &mut Handle<Image>, &mut Owner)>,
) {
    for evt in flip.iter() {
        let mut debug_found_card_in_evt = false;
        for (&PlacedCard(cell), mut image, mut owner) in &mut query {
            if cell == evt.cell as usize {
                *image = if *image == app_assets.card_bg_red {
                    owner.0 = core::Player::P1;
                    app_assets.card_bg_blue.clone()
                } else {
                    owner.0 = core::Player::P2;
                    app_assets.card_bg_red.clone()
                };
                debug_found_card_in_evt = true;
                break;
            }
        }
        debug_assert!(
            debug_found_card_in_evt,
            "Card in Flip event not a PlacedCard"
        );
    }
}

fn handle_battle(
    mut commands: Commands,
    mut battle: EventReader<Battle>,
    app_assets: Res<AppAssets>,
) {
    for evt in battle.iter() {
        spawn_battler_stats(&mut commands, &app_assets, evt.attacker);
        spawn_battler_stats(&mut commands, &app_assets, evt.defender);
    }
}

fn update_card_counter(
    app_assets: Res<AppAssets>,
    mut red: Query<(&mut RedCardCounter, &mut Handle<Image>), Without<BlueCardCounter>>,
    mut blue: Query<(&mut BlueCardCounter, &mut Handle<Image>), Without<RedCardCounter>>,
    placed_cards: Query<&Owner, Added<PlacedCard>>,
    flipped_cards: Query<&Owner, (Changed<Owner>, With<PlacedCard>)>,
) {
    let (mut red_count, mut red_image) = red.single_mut();
    let (mut blue_count, mut blue_image) = blue.single_mut();

    // update on placing cards
    for &Owner(owner) in &placed_cards {
        match owner {
            core::Player::P1 => {
                blue_count.0 += 1;
            }
            core::Player::P2 => {
                red_count.0 += 1;
            }
        }
    }

    // update on flipping cards
    for &Owner(owner) in &flipped_cards {
        match owner {
            core::Player::P1 => {
                blue_count.0 += 1;
                red_count.0 -= 1;
            }
            core::Player::P2 => {
                blue_count.0 -= 1;
                red_count.0 += 1;
            }
        }
    }

    // update the textures
    *red_image = app_assets.card_counter_red[red_count.0].clone();
    *blue_image = app_assets.card_counter_blue[blue_count.0].clone();
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

pub(crate) fn spawn_battler_stats(
    commands: &mut Commands,
    app_assets: &AppAssets,
    battler: core::Battler,
) {
    const WIDTH_1_DIGIT_1_POS: Vec2 = Vec2::new(16., 24.);

    const WIDTH_2_DIGIT_1_POS: Vec2 = Vec2::new(11., 24.);
    const WIDTH_2_DIGIT_2_POS: Vec2 = Vec2::new(21., 24.);

    const WIDTH_3_DIGIT_1_POS: Vec2 = Vec2::new(6., 24.);
    const WIDTH_3_DIGIT_2_POS: Vec2 = Vec2::new(16., 24.);
    const WIDTH_3_DIGIT_3_POS: Vec2 = Vec2::new(26., 24.);

    fn spawn_digit(app_assets: &AppAssets, p: &mut ChildBuilder<'_, '_, '_>, index: u8, pos: Vec2) {
        p.spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: index as usize,
                anchor: Anchor::BottomLeft,
                ..default()
            },
            texture_atlas: app_assets.battle_digits.clone(),
            transform: Transform::from_xyz(pos.x, pos.y, 0.),
            ..default()
        });
    }

    let mut position = calc_board_card_screen_pos(battler.cell as usize);
    position.z += 1.;
    let transform = Transform::from_translation(position);
    commands
        .spawn()
        .insert(BattlerStatDisplay)
        .insert_bundle(TransformBundle::from_transform(transform))
        .insert(Visibility::default())
        .insert(ComputedVisibility::default())
        .insert(Cleanup)
        .with_children(|p| {
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
            p.spawn_bundle(SpriteSheetBundle {
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
