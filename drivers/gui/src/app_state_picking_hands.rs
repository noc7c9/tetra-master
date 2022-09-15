use crate::{card, game_state, AppAssets, AppState, RENDER_HSIZE};
use bevy::prelude::*;
use tetra_master_core as core;

const CARD_SIZE: Vec2 = Vec2::new(42., 51.);

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
                    .with_system(hover)
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

// stores the index into the hand
#[derive(Debug, Component, Clone, Copy)]
struct CardIdx(usize);

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
                .insert(CardIdx(card_idx))
                .insert(HandIdx(hand_idx));
        }

        let pos = (x, y).into();
        let size = (CARD_SIZE.x * 5. + CENTER_HAND_PADDING * 4., CARD_SIZE.y).into();
        commands
            .spawn()
            .insert(Cleanup)
            .insert(Hoverable::new(pos, size))
            .insert(HandIdx(hand_idx));
    }
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<HoveredHand>();
}

fn pick_hand(
    mut commands: Commands,
    mut game_state: ResMut<game_state::picking_hands::State>,
    hovered_hand: ResMut<HoveredHand>,
    btns: Res<Input<MouseButton>>,
    mut hand_cards: Query<(Entity, &mut card::Owner, &mut Transform, &CardIdx, &HandIdx)>,
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

                            // remove hand candidate marker
                            commands.entity(entity).remove::<HandIdx>();
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

                            // remove hand candidate marker
                            commands.entity(entity).remove::<HandIdx>();
                        }
                        // part of the unpicked hand
                        else {
                            // so remove it
                            commands.entity(entity).despawn_recursive();
                        }
                    }

                    // forward the game state
                    game_state.pick_hand(picked_hand as usize);
                }
                game_state::picking_hands::Status::Done { .. } => {
                    unreachable!("Both hands already picked")
                }
            }
        }
    }
}

#[derive(Debug, Component)]
struct Hoverable {
    is_hovered: bool,
    bounding_box: (Vec2, Vec2),
}

impl Hoverable {
    fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            is_hovered: false,
            bounding_box: (position, position + size),
        }
    }

    fn contains(&self, point: Vec2) -> bool {
        let a = self.bounding_box.0;
        let b = self.bounding_box.1;
        (a.x..b.x).contains(&point.x) && (a.y..b.y).contains(&point.y)
    }
}

fn hover(
    mut cursor_moved: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut hoverables: Query<&mut Hoverable>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.single();
    let window = windows.get_primary().unwrap();

    let evt = match cursor_moved.iter().last() {
        None => return,
        Some(evt) => evt,
    };

    let screen_pos = evt.position;
    let screen_size = Vec2::new(window.width() as f32, window.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / screen_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    let world_pos: Vec2 = world_pos.truncate();

    for mut hoverable in &mut hoverables {
        let is_hovered = hoverable.contains(world_pos);
        // avoid triggering change detection if it's the same
        if hoverable.is_hovered != is_hovered {
            hoverable.is_hovered = is_hovered;
        }
    }
}

fn highlight_on_hover(
    mut prev_hovered_hand: ResMut<HoveredHand>,
    game_state: Res<game_state::picking_hands::State>,
    mut hand_cards: Query<(&mut card::Owner, &HandIdx)>,
    changed: Query<(Entity, &Hoverable, &HandIdx), Changed<Hoverable>>,
) {
    // nothing changed, so nothing to do
    if changed.is_empty() {
        return;
    }

    // picking is done, so nothing to do
    if let game_state::picking_hands::Status::Done { .. } = game_state.status {
        return;
    }

    // figure out which hand is currently hovered over (if any)
    let curr_hovered_hand = changed.iter().find_map(|(entity, hoverable, hand)| {
        if hoverable.is_hovered {
            Some((entity, hand.0))
        } else {
            None
        }
    });

    // hovered hand has changed
    if prev_hovered_hand.0 != curr_hovered_hand {
        prev_hovered_hand.0 = curr_hovered_hand;

        // (un)highlight all cards based on which hand is hovered over
        for (mut owner, hand) in &mut hand_cards {
            if Some(hand.0) == curr_hovered_hand.map(|(_, idx)| idx) {
                owner.0 = Some(game_state.picking_player());
            } else {
                owner.0 = None;
            }
        }
    }
}
