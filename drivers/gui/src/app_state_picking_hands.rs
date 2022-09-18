use crate::{card, game_state, hover, AppAssets, AppState, CARD_SIZE, RENDER_HSIZE};
use bevy::prelude::*;
use tetra_master_core as core;

const PLAYER_HAND_VOFFSET: f32 = 27.;

const PLAYER_HAND_PADDING: f32 = 4.;
const CENTER_HAND_PADDING: f32 = 3.;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::PickingHands).with_system(on_enter))
            .add_system_set(
                SystemSet::on_update(AppState::PickingHands)
                    .with_system(pick_hand)
                    .with_system(highlight_on_hover),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::PickingHands)
                    .with_system(on_exit)
                    .with_system(crate::cleanup::<Cleanup>),
            );
    }
}

struct HoveredHand(Option<(Entity, usize)>);

#[derive(Component)]
struct Cleanup;

// stores the index into the hand_candidates array
#[derive(Debug, Component, Clone, Copy)]
struct HandIdx(usize);

fn on_enter(
    mut commands: Commands,
    app_assets: Res<AppAssets>,
    game_state: Res<game_state::picking_hands::State>,
) {
    commands.insert_resource(HoveredHand(None));

    for (hand_idx, hand) in game_state.candidates.iter().enumerate() {
        let hand = hand.unwrap(); // will all exist on_enter
        let i = hand_idx as f32;
        let y = CARD_SIZE.y * 0.5 + CENTER_HAND_PADDING + i * -(CARD_SIZE.y + CENTER_HAND_PADDING);
        let x = CARD_SIZE.x * -2.5 + CENTER_HAND_PADDING * -2.;
        for (card_idx, card) in hand.iter().enumerate() {
            let j = card_idx as f32;
            let x = x + j * (CARD_SIZE.x + CENTER_HAND_PADDING);
            let z = 1. + i * 5. + j;
            card::spawn(&mut commands, &app_assets, (x, y, z).into(), None, *card)
                .insert(Cleanup)
                .insert(card::CardIdx(card_idx))
                .insert(HandIdx(hand_idx));
        }

        let size = (CARD_SIZE.x * 5. + CENTER_HAND_PADDING * 4., CARD_SIZE.y).into();
        let transform = Transform::from_xyz(x, y, 0.);
        commands
            .spawn()
            .insert(Cleanup)
            .insert_bundle(TransformBundle::from_transform(transform))
            .insert(hover::Area::new(size))
            // .insert(crate::debug::rect(size))
            .insert(HandIdx(hand_idx));
    }
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<HoveredHand>();
}

fn pick_hand(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut game_state: ResMut<game_state::picking_hands::State>,
    hovered_hand: ResMut<HoveredHand>,
    btns: Res<Input<MouseButton>>,
    mut hand_cards: Query<(
        Entity,
        &mut card::Owner,
        &mut Transform,
        &card::CardIdx,
        &HandIdx,
    )>,
) {
    let hovered_hand = hovered_hand.into_inner();
    if let Some((hoverable_entity, picked_hand)) = hovered_hand.0 {
        if btns.just_pressed(MouseButton::Left) {
            match game_state.status {
                game_state::picking_hands::Status::BluePicking { .. } => {
                    // remove the hoverable entity
                    commands.entity(hoverable_entity).despawn_recursive();
                    hovered_hand.0 = None;

                    // iterate over each of the cards in a hand candidate
                    for (entity, mut owner, mut transform, card_idx, hand_idx) in &mut hand_cards {
                        // this card is part of the picked hand
                        if hand_idx.0 == picked_hand {
                            // move it to the blue side
                            owner.0 = Some(core::Player::P1);
                            transform.translation.x =
                                RENDER_HSIZE.x - CARD_SIZE.x - PLAYER_HAND_PADDING;
                            transform.translation.y = RENDER_HSIZE.y
                                - CARD_SIZE.y
                                - PLAYER_HAND_PADDING
                                - PLAYER_HAND_VOFFSET * card_idx.0 as f32;

                            // remove the hand candidate marker and the clean up marker
                            commands
                                .entity(entity)
                                .remove::<HandIdx>()
                                .remove::<Cleanup>();
                        }
                        // TODO: recenter remaining two candidates
                        // else {
                        // }
                    }

                    // forward the game state
                    game_state.pick_hand(picked_hand as usize);
                }
                game_state::picking_hands::Status::RedPicking { .. } => {
                    // remove the hoverable entity
                    commands.entity(hoverable_entity).despawn_recursive();
                    hovered_hand.0 = None;

                    // iterate over each of the cards in a hand candidate
                    for (entity, mut owner, mut transform, card_idx, hand_idx) in &mut hand_cards {
                        // this card is part of the picked hand
                        if hand_idx.0 == picked_hand {
                            // move it to the red side
                            owner.0 = Some(core::Player::P2);
                            transform.translation.x = -RENDER_HSIZE.x + PLAYER_HAND_PADDING;
                            transform.translation.y = RENDER_HSIZE.y
                                - CARD_SIZE.y
                                - PLAYER_HAND_PADDING
                                - PLAYER_HAND_VOFFSET * card_idx.0 as f32;

                            // remove the hand candidate marker and the clean up marker
                            commands
                                .entity(entity)
                                .remove::<HandIdx>()
                                .remove::<Cleanup>();
                        }
                        // part of the unpicked hand
                        else {
                            // so remove it
                            commands.entity(entity).despawn_recursive();
                        }
                    }

                    // forward the game state
                    game_state.pick_hand(picked_hand as usize);

                    // forward the app state
                    let _ = app_state.set(AppState::InGame);
                }
                game_state::picking_hands::Status::Done { .. } => {
                    unreachable!("Both hands already picked")
                }
            }
        }
    }
}

fn highlight_on_hover(
    mut hovered_hand: ResMut<HoveredHand>,
    game_state: Res<game_state::picking_hands::State>,
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    mut hand_cards: Query<(&mut card::Owner, &HandIdx)>,
    hands: Query<&HandIdx>,
) {
    // picking is done, so nothing to do
    if let game_state::picking_hands::Status::Done { .. } = game_state.status {
        return;
    }

    let mut set_highlight = |hand_idx, highlight| {
        for (mut owner, hand) in &mut hand_cards {
            if hand.0 == hand_idx {
                owner.0 = highlight;
            }
        }
    };

    for evt in hover_end.iter() {
        let hand_idx = hands.get(evt.entity).unwrap();
        set_highlight(hand_idx.0, None);

        hovered_hand.0 = None;
    }
    for evt in hover_start.iter() {
        let hand_idx = hands.get(evt.entity).unwrap();
        set_highlight(hand_idx.0, Some(game_state.picking_player()));

        hovered_hand.0 = Some((evt.entity, hand_idx.0));
    }
}
