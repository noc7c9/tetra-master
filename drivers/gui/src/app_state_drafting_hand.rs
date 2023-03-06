use crate::{
    common::{spawn_card, start_new_game, Card},
    hover,
    layout::{self, TransformExt as _, Z},
    random_setup_generator::random_card as generate_random_card,
    AppAssets, AppState, CARD_ASSET_SIZE, CARD_SIZE,
};
use bevy::prelude::*;
use tetra_master_core as core;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        let enter = SystemSet::on_enter(AppState::DraftingHand).with_system(setup);

        let update =
            SystemSet::on_update(AppState::DraftingHand).with_system(handle_keyboard_input);
        // .with_system(handle_candidate_hover)
        // .with_system(handle_mouse_clicks);

        let exit =
            SystemSet::on_exit(AppState::DraftingHand).with_system(crate::cleanup::<Cleanup>);

        app.add_system_set(enter)
            .add_system_set(update)
            .add_system_set(exit);
    }
}

#[derive(Component)]
struct Cleanup;

#[derive(Resource, Default)]
struct CardChoices {
    hand: [Option<Card>; core::HAND_SIZE],
    side_board: [Option<Card>; 2],
}

// #[derive(Debug, Deref, Resource)]
// struct HoveredCandidate(Option<Entity>);

// #[derive(Debug, Deref, Resource)]
// struct ActiveCandidate(Option<Entity>);

#[derive(Component)]
struct Candidate(usize);

// #[derive(Resource)]
// struct ActiveMenuOption(Option<MenuOption>);

// #[derive(Component, Clone, Copy)]
// enum MenuOption {
//     Draft,
//     Constructed,
//     Collection,
// }

// impl MenuOption {
//     const fn text(self) -> &'static str {
//         match self {
//             MenuOption::Draft => "Draft",
//             MenuOption::Constructed => "Constructed",
//             MenuOption::Collection => "Collection",
//         }
//     }

//     const fn offset_y(self) -> f32 {
//         match self {
//             MenuOption::Draft => -150.0,
//             MenuOption::Constructed => -220.0,
//             MenuOption::Collection => -290.0,
//         }
//     }

//     const fn width(self) -> f32 {
//         match self {
//             MenuOption::Draft => 135.0,
//             MenuOption::Constructed => 300.0,
//             MenuOption::Collection => 235.0,
//         }
//     }
// }

// #[derive(Component)]
// struct Cursor;

// #[derive(Component)]
// struct AnimationTimer(Timer);

fn setup(mut commands: Commands, app_assets: Res<AppAssets>) {
    commands.insert_resource(CardChoices::default());
    // commands.insert_resource(HoveredCandidate(None));
    // commands.insert_resource(ActiveCandidate(None));

    commands.spawn((
        Cleanup,
        crate::debug::text("Press 1, 2, or 3 to select a card to draft."),
    ));

    // let vertical_offset = -180.;

    // commands.spawn((
    //     SpriteBundle {
    //         texture: app_assets.hand_slots.clone(),
    //         transform: layout::left()
    //             .z(Z::BG + 0.1)
    //             .offset_x(CARD_SIZE.x * 2.5)
    //             .offset((30., vertical_offset)),
    //         ..default()
    //     },
    //     Cleanup,
    // ));

    // let transform = layout::right()
    //     .z(Z::BG + 0.1)
    //     .offset_x(-CARD_SIZE.x / 2.)
    //     .offset((-30., vertical_offset));
    // commands.spawn((
    //     SpriteBundle {
    //         texture: app_assets.card_slot.clone(),
    //         transform,
    //         ..default()
    //     },
    //     Cleanup,
    // ));
    // commands.spawn((
    //     SpriteBundle {
    //         texture: app_assets.card_slot.clone(),
    //         transform: transform.offset_x(-CARD_SIZE.x - 4.),
    //         ..default()
    //     },
    //     Cleanup,
    // ));

    spawn_new_candidates(&mut commands, &app_assets);

    //     commands.insert_resource(ActiveMenuOption(None));

    //     let transform = layout::center().z(Z::UI_TEXT).scale(1.);
    //     for menu_option in [
    //         MenuOption::Draft,
    //         MenuOption::Constructed,
    //         MenuOption::Collection,
    //     ] {
    //         let text = |color, text| {
    //             let style = TextStyle {
    //                 font: app_assets.font.clone(),
    //                 font_size: 60.0,
    //                 color,
    //             };
    //             Text::from_section(text, style).with_alignment(TextAlignment::CENTER)
    //         };

    //         let transform = transform.offset_y(menu_option.offset_y()).offset_x(2.5);
    //         commands
    //             .spawn((
    //                 Text2dBundle {
    //                     text: text(Color::WHITE, menu_option.text()),
    //                     transform: transform.offset_z(0.1),
    //                     ..default()
    //                 },
    //                 Cleanup,
    //             ))
    //             .with_children(|p| {
    //                 // text shadow
    //                 p.spawn(Text2dBundle {
    //                     text: text(Color::hex("383840").unwrap(), menu_option.text()),
    //                     transform: Transform::from_xyz(4., -4., 0.0),
    //                     ..default()
    //                 });

    //                 // hover area
    //                 let hover_area_size = Vec2::new(menu_option.width(), 50.);
    //                 let transform = Transform::from_xyz(0., -5., 0.);
    //                 p.spawn((
    //                     menu_option,
    //                     TransformBundle::from_transform(transform),
    //                     hover::Area::new(hover_area_size),
    //                     // crate::debug::rect(hover_area_size),
    //                 ));
    //             });
    //     }

    //     commands.spawn((
    //         SpriteSheetBundle {
    //             sprite: TextureAtlasSprite {
    //                 index: 0,
    //                 ..default()
    //             },
    //             texture_atlas: app_assets.cursor.clone(),
    //             transform: layout::center().z(Z::UI_TEXT),
    //             visibility: Visibility::INVISIBLE,
    //             ..default()
    //         },
    //         Cursor,
    //         AnimationTimer(Timer::from_seconds(0.08, TimerMode::Repeating)),
    //         Cleanup,
    //     ));
    //     commands.spawn((
    //         SpriteSheetBundle {
    //             sprite: TextureAtlasSprite {
    //                 index: 0,
    //                 flip_x: true,
    //                 ..default()
    //             },
    //             texture_atlas: app_assets.cursor.clone(),
    //             transform: layout::center().z(Z::UI_TEXT),
    //             visibility: Visibility::INVISIBLE,
    //             ..default()
    //         },
    //         Cursor,
    //         AnimationTimer(Timer::from_seconds(0.08, TimerMode::Repeating)),
    //         Cleanup,
    //     ));
}

fn spawn_new_candidates(commands: &mut Commands, app_assets: &AppAssets) {
    let mut rng = core::Rng::new();

    let candidates = [
        generate_random_card(&mut rng),
        generate_random_card(&mut rng),
        generate_random_card(&mut rng),
    ];

    for (idx, card) in candidates.iter().enumerate() {
        let transform = calc_transform_for_candidate_card(idx);
        spawn_card(commands, app_assets, transform, *card, None).insert((
            // hover::Area::new(CARD_ASSET_SIZE),
            // crate::debug::rect(CARD_ASSET_SIZE),
            Candidate(idx),
            Cleanup,
        ));
        // .insert(OptionalOwner(None))
        // .insert(Owner(owner))
        // .insert(Cleanup)
        // .insert(HandIdx(hand_idx));
        // .insert(CandidateIdx(candidate_idx));
    }
}

fn calc_transform_for_candidate_card(idx: usize) -> Transform {
    let pos = layout::center().offset_y(CARD_SIZE.y / 2. + 60.).z(1.);

    layout::line_horizontal(pos)
        .num_entities(3)
        .entity_size(CARD_SIZE)
        .padding(CARD_SIZE.x + 4.)
        .index(idx)

    // .offset_x(CARD_SIZE.x * 2.5)
    // .offset_y(CARD_SIZE.y / 2.)
    // .offset((30., 60.))
    // .offset_x(hand_idx as f32 * (CARD_SIZE.x + 4.))
    // .offset((0., 0.))
    // .position()
}

fn handle_keyboard_input(keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Key1) {
        log::info!("1 pressed");
    }
    if keyboard_input.just_pressed(KeyCode::Key2) {
        log::info!("2 pressed");
    }
    if keyboard_input.just_pressed(KeyCode::Key3) {
        log::info!("3 pressed");
    }
}

// fn handle_candidate_hover(
//     mut hover_end: EventReader<hover::EndEvent>,
//     mut hover_start: EventReader<hover::StartEvent>,
//     mut hovered_candidate: ResMut<HoveredCandidate>,
//     app_assets: Res<AppAssets>,
//     mut candidates: Query<&mut Handle<Image>, With<Candidate>>,
// ) {
//     for evt in hover_end.iter() {
//         if let Ok(mut texture) = candidates.get_mut(evt.entity) {
//             if hovered_candidate.0 == Some(evt.entity) {
//                 hovered_candidate.0 = None;

//                 // set card texture back to gray
//                 *texture = app_assets.card_bg_gray.clone();
//             }
//         }
//     }

//     for evt in hover_start.iter() {
//         if let Ok(mut texture) = candidates.get_mut(evt.entity) {
//             hovered_candidate.0 = Some(evt.entity);

//             // set card texture to blue
//             *texture = app_assets.card_bg_blue.clone();
//         }
//     }
// }

// fn handle_mouse_clicks(
//     btns: Res<Input<MouseButton>>,
//     mut hovered_candidate: ResMut<HoveredCandidate>,
//     candidates: Query<&Candidate>,
// ) {
//     if btns.just_pressed(MouseButton::Left) {
//         // if this click is on a candidate
//         if let Some(Candidate(candidate)) =
//             hovered_candidate.and_then(|entity| candidates.get(entity).ok())
//         {
//             println!("clicked on candidate: {candidate}");
//         }
//     }
// }

// fn detect_hover(
//     mut active_menu_option: ResMut<ActiveMenuOption>,
//     mut hover_end: EventReader<hover::EndEvent>,
//     mut hover_start: EventReader<hover::StartEvent>,
//     mut cursor: Query<(&mut Visibility, &mut Transform), With<Cursor>>,
//     menu_options: Query<&MenuOption>,
// ) {
//     for _ in hover_end.iter() {
//         for mut cursor in &mut cursor {
//             cursor.0.is_visible = false;
//         }
//         active_menu_option.0 = None;
//     }

//     for evt in hover_start.iter() {
//         if let Ok(menu_option) = menu_options.get(evt.entity) {
//             for (idx, mut cursor) in cursor.iter_mut().enumerate() {
//                 cursor.0.is_visible = true;

//                 let offset_x = menu_option.width() / 2. + CURSOR_SIZE.x / 2. + 20.;
//                 cursor.1.translation.x = offset_x * (idx as f32 * 2. - 1.);

//                 cursor.1.translation.y = menu_option.offset_y() - 5.;
//             }

//             active_menu_option.0 = Some(*menu_option);
//         }
//     }
// }

// fn mouse_input(
//     mut commands: Commands,
//     mut app_state: ResMut<State<AppState>>,
//     mut btns: ResMut<Input<MouseButton>>,
//     args: Res<crate::Args>,
//     active_menu_option: Res<ActiveMenuOption>,
// ) {
//     if btns.just_pressed(MouseButton::Left)
//         && matches!(active_menu_option.0, Some(MenuOption::Draft))
//     {
//         start_new_game(&mut commands, &mut app_state, &args);

//         // required to workaround bug?
//         btns.reset(MouseButton::Left);
//     }
// }

// fn animate_cursor(
//     time: Res<Time>,
//     mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite), With<Cursor>>,
// ) {
//     for (mut timer, mut sprite) in &mut query {
//         timer.0.tick(time.delta());
//         if timer.0.just_finished() {
//             sprite.index = (sprite.index + 1) % 8;
//         }
//     }
// }
