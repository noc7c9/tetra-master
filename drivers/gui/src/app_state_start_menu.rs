use super::{AppAssets, AppState};
use bevy::prelude::*;

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
        commands.insert_resource(crate::game_state::picking_hands::State::new(
            &args.implementation,
        ));

        // change the state
        let _ = app_state.set(AppState::PickingHands);

        // required to workaround bug?
        btns.reset(MouseButton::Left);
    }
}