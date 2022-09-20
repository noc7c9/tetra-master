use super::{
    common::{Candidates, Driver, Turn},
    AppAssets, AppState,
};
use bevy::prelude::*;
use tetra_master_core as core;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::StartMenu).with_system(setup))
            .add_system_set(SystemSet::on_update(AppState::StartMenu).with_system(mouse_input))
            .add_system_set(
                SystemSet::on_exit(AppState::StartMenu).with_system(crate::cleanup::<Cleanup>),
            );
    }
}

#[derive(Component)]
struct Cleanup;

fn setup(mut commands: Commands, app_assets: Res<AppAssets>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .insert(Cleanup)
        .with_children(|parent| {
            parent.spawn_bundle(
                TextBundle::from_section(
                    "Left Click to Start a New Game!",
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
}

fn mouse_input(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut btns: ResMut<Input<MouseButton>>,
    args: Res<crate::Args>,
) {
    if btns.just_pressed(MouseButton::Left) {
        // start the new game
        let mut driver = core::Driver::new(&args.implementation).log();
        let cmd = core::command::Setup {
            rng: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        };
        // TODO: handle the error
        let response = driver.send(cmd).unwrap();
        let c = response.hand_candidates;
        commands.insert_resource(Candidates([Some(c[0]), Some(c[1]), Some(c[2])]));

        commands.insert_resource(Turn(core::Player::P1));

        commands.insert_resource(Driver(driver));

        // change the state
        app_state.set(AppState::PickingHands).unwrap();

        // required to workaround bug?
        btns.reset(MouseButton::Left);
    }
}
