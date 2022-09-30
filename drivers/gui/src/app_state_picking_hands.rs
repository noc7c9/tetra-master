use crate::{
    common::{
        calc_candidate_card_screen_pos, calc_hand_card_screen_pos, spawn_card, z_index, Candidates,
        Driver, HandIdx, OptionalOwner, Owner, CANDIDATE_PADDING,
    },
    hover, AppAssets, AppState, CARD_SIZE,
};
use bevy::prelude::*;
use tetra_master_core as core;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::PickingHands).with_system(on_enter))
            .add_system_set(
                SystemSet::on_update(AppState::PickingHands)
                    .with_system(pick_hand)
                    .with_system(highlight_on_hover)
                    .with_system(swap_card_background),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::PickingHands)
                    .with_system(on_exit)
                    .with_system(crate::cleanup::<Cleanup>),
            );
    }
}

enum Status {
    BluePicking,
    RedPicking,
}

struct HoveredCandidate(Option<(Entity, usize)>);

#[derive(Component)]
struct Cleanup;

// stores the index into the hand_candidates array
#[derive(Debug, Component, Clone, Copy)]
struct CandidateIdx(usize);

fn on_enter(mut commands: Commands, app_assets: Res<AppAssets>, candidates: Res<Candidates>) {
    commands.insert_resource(Status::BluePicking);
    commands.insert_resource(HoveredCandidate(None));

    for (candidate_idx, candidate) in candidates.0.iter().enumerate() {
        let candidate = candidate.unwrap(); // will all exist on_enter
        for (hand_idx, card) in candidate.iter().enumerate() {
            let position = calc_candidate_card_screen_pos(candidate_idx, hand_idx);
            spawn_card(&mut commands, &app_assets, position, *card)
                .insert(OptionalOwner(None))
                .insert(Cleanup)
                .insert(HandIdx(hand_idx))
                .insert(CandidateIdx(candidate_idx));
        }

        let size = (CARD_SIZE.x * 5. + CANDIDATE_PADDING * 4., CARD_SIZE.y).into();
        let mut position = calc_candidate_card_screen_pos(candidate_idx, 0);
        position.z = z_index::CANDIDATE_HAND_HOVER_AREA;
        let transform = Transform::from_translation(position);
        commands
            .spawn()
            .insert(Cleanup)
            .insert_bundle(TransformBundle::from_transform(transform))
            .insert(hover::Area::new(size))
            // .insert(crate::debug::rect(size))
            .insert(CandidateIdx(candidate_idx));
    }
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<Status>();
    commands.remove_resource::<HoveredCandidate>();
}

fn pick_hand(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut driver: ResMut<Driver>,
    mut status: ResMut<Status>,
    hovered_candidate: ResMut<HoveredCandidate>,
    btns: Res<Input<MouseButton>>,
    mut cards: Query<(Entity, &mut Transform, &HandIdx, &CandidateIdx)>,
) {
    let hovered_candidate = hovered_candidate.into_inner();
    if let Some((hoverable_entity, picked_candidate)) = hovered_candidate.0 {
        if btns.just_pressed(MouseButton::Left) {
            match *status {
                Status::BluePicking => {
                    // remove the hoverable entity
                    commands.entity(hoverable_entity).despawn_recursive();
                    hovered_candidate.0 = None;

                    // iterate over each of the cards in a hand candidate
                    for (entity, mut transform, hand_idx, candidate_idx) in &mut cards {
                        // this card is part of the picked hand
                        if candidate_idx.0 == picked_candidate {
                            // move it to the blue side
                            transform.translation =
                                calc_hand_card_screen_pos(core::Player::P1, hand_idx.0);

                            commands
                                .entity(entity)
                                // replace OptionalOwner with Owner
                                .remove::<OptionalOwner>()
                                .insert(Owner(core::Player::P1))
                                // remove the hand candidate marker and the clean up marker
                                .remove::<CandidateIdx>()
                                .remove::<Cleanup>();
                        }
                        // TODO: recenter remaining two candidates
                        // else {
                        // }
                    }

                    // forward the game state
                    driver
                        .0
                        .send(core::command::PickHand {
                            hand: picked_candidate as u8,
                        })
                        .expect("PickHand command should work");

                    *status = Status::RedPicking;
                }
                Status::RedPicking => {
                    // remove the hoverable entity
                    commands.entity(hoverable_entity).despawn_recursive();
                    hovered_candidate.0 = None;

                    // iterate over each of the cards in a hand candidate
                    for (entity, mut transform, hand_idx, candidate_idx) in &mut cards {
                        // this card is part of the picked hand
                        if candidate_idx.0 == picked_candidate {
                            // move it to the red side
                            transform.translation =
                                calc_hand_card_screen_pos(core::Player::P2, hand_idx.0);

                            commands
                                .entity(entity)
                                // replace OptionalOwner with Owner
                                .remove::<OptionalOwner>()
                                .insert(Owner(core::Player::P2))
                                // remove the hand candidate marker and the clean up marker
                                .remove::<CandidateIdx>()
                                .remove::<Cleanup>();
                        }
                        // part of the unpicked hand
                        else {
                            // so remove it
                            commands.entity(entity).despawn_recursive();
                        }
                    }

                    // forward the game state
                    driver
                        .0
                        .send(core::command::PickHand {
                            hand: picked_candidate as u8,
                        })
                        .expect("PickHand command should work");

                    // forward the app state
                    let _ = app_state.set(AppState::InGame);
                }
            }
        }
    }
}

fn highlight_on_hover(
    mut hovered_candidate: ResMut<HoveredCandidate>,
    status: Res<Status>,
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    mut cards: Query<(&mut OptionalOwner, &CandidateIdx)>,
    candidates: Query<&CandidateIdx>,
) {
    let mut set_highlight = |candidate_idx, highlight| {
        for (mut owner, hand) in &mut cards {
            if hand.0 == candidate_idx {
                owner.0 = highlight;
            }
        }
    };

    for evt in hover_end.iter() {
        // might be missing if this triggers during clean up
        if let Ok(candidate_idx) = candidates.get(evt.entity) {
            set_highlight(candidate_idx.0, None);

            hovered_candidate.0 = None;
        }
    }
    for evt in hover_start.iter() {
        // might be missing if this triggers during clean up
        if let Ok(candidate_idx) = candidates.get(evt.entity) {
            let highlight = match *status {
                Status::BluePicking => core::Player::P1,
                Status::RedPicking => core::Player::P2,
            };
            set_highlight(candidate_idx.0, Some(highlight));

            hovered_candidate.0 = Some((evt.entity, candidate_idx.0));
        }
    }
}

fn swap_card_background(
    app_assets: Res<AppAssets>,
    mut query: Query<(&mut Handle<Image>, &OptionalOwner), Changed<OptionalOwner>>,
) {
    for (mut texture, owner) in &mut query {
        *texture = match owner.0 {
            None => app_assets.card_bg_gray.clone(),
            Some(core::Player::P1) => app_assets.card_bg_blue.clone(),
            Some(core::Player::P2) => app_assets.card_bg_red.clone(),
        };
    }
}
