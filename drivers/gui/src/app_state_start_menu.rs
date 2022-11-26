use super::{
    common::start_new_game,
    layout::{self, TransformExt as _, Z},
    AppAssets, AppState,
};
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
    let style = TextStyle {
        font: app_assets.font.clone(),
        font_size: 40.0,
        color: Color::WHITE,
    };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Left Click to Start a New Game!", style)
                .with_alignment(TextAlignment::CENTER),
            transform: layout::bottom().z(Z::UI_TEXT).scale(1.).offset_y(160.),
            ..default()
        },
        Cleanup,
    ));
}

fn mouse_input(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut btns: ResMut<Input<MouseButton>>,
    args: Res<crate::Args>,
) {
    if btns.just_pressed(MouseButton::Left) {
        start_new_game(&mut commands, &mut app_state, &args);

        // required to workaround bug?
        btns.reset(MouseButton::Left);
    }
}
