use crate::{
    common::start_new_game,
    hover,
    layout::{self, TransformExt as _, Z},
    AppAssets, AppState, CURSOR_SIZE,
};
use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::StartMenu).with_system(setup))
            .add_system_set(
                SystemSet::on_update(AppState::StartMenu)
                    .with_system(detect_hover)
                    .with_system(mouse_input)
                    .with_system(animate_cursor),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::StartMenu).with_system(crate::cleanup::<Cleanup>),
            );
    }
}

#[derive(Component)]
struct Cleanup;

#[derive(Resource)]
struct ActiveMenuOption(Option<MenuOption>);

#[derive(Component, Clone, Copy)]
enum MenuOption {
    Draft,
    Constructed,
    Collection,
}

impl MenuOption {
    const fn text(self) -> &'static str {
        match self {
            MenuOption::Draft => "Draft",
            MenuOption::Constructed => "Constructed",
            MenuOption::Collection => "Collection",
        }
    }

    const fn offset_y(self) -> f32 {
        match self {
            MenuOption::Draft => -150.0,
            MenuOption::Constructed => -220.0,
            MenuOption::Collection => -290.0,
        }
    }

    const fn width(self) -> f32 {
        match self {
            MenuOption::Draft => 135.0,
            MenuOption::Constructed => 300.0,
            MenuOption::Collection => 235.0,
        }
    }
}

#[derive(Component)]
struct Cursor;

#[derive(Component)]
struct AnimationTimer(Timer);

fn setup(mut commands: Commands, app_assets: Res<AppAssets>) {
    commands.insert_resource(ActiveMenuOption(None));

    let transform = layout::center().z(Z::UI_TEXT).scale(1.);
    for menu_option in [
        MenuOption::Draft,
        MenuOption::Constructed,
        MenuOption::Collection,
    ] {
        let text = |color, text| {
            let style = TextStyle {
                font: app_assets.font.clone(),
                font_size: 60.0,
                color,
            };
            Text::from_section(text, style).with_alignment(TextAlignment::CENTER)
        };

        let transform = transform.offset_y(menu_option.offset_y()).offset_x(2.5);
        commands
            .spawn((
                Text2dBundle {
                    text: text(Color::WHITE, menu_option.text()),
                    transform: transform.offset_z(0.1),
                    ..default()
                },
                Cleanup,
            ))
            .with_children(|p| {
                // text shadow
                p.spawn(Text2dBundle {
                    text: text(Color::hex("383840").unwrap(), menu_option.text()),
                    transform: Transform::from_xyz(4., -4., 0.0),
                    ..default()
                });

                // hover area
                let hover_area_size = Vec2::new(menu_option.width(), 50.);
                let transform = Transform::from_xyz(0., -5., 0.);
                p.spawn((
                    menu_option,
                    TransformBundle::from_transform(transform),
                    hover::Area::new(hover_area_size),
                    // crate::debug::rect(hover_area_size),
                ));
            });
    }

    commands.spawn((
        SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: 0,
                ..default()
            },
            texture_atlas: app_assets.cursor.clone(),
            transform: layout::center().z(Z::UI_TEXT),
            visibility: Visibility::INVISIBLE,
            ..default()
        },
        Cursor,
        AnimationTimer(Timer::from_seconds(0.08, TimerMode::Repeating)),
        Cleanup,
    ));
    commands.spawn((
        SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: 0,
                flip_x: true,
                ..default()
            },
            texture_atlas: app_assets.cursor.clone(),
            transform: layout::center().z(Z::UI_TEXT),
            visibility: Visibility::INVISIBLE,
            ..default()
        },
        Cursor,
        AnimationTimer(Timer::from_seconds(0.08, TimerMode::Repeating)),
        Cleanup,
    ));
}

fn detect_hover(
    mut active_menu_option: ResMut<ActiveMenuOption>,
    mut hover_end: EventReader<hover::EndEvent>,
    mut hover_start: EventReader<hover::StartEvent>,
    mut cursor: Query<(&mut Visibility, &mut Transform), With<Cursor>>,
    menu_options: Query<&MenuOption>,
) {
    for _ in hover_end.iter() {
        for mut cursor in &mut cursor {
            cursor.0.is_visible = false;
        }
        active_menu_option.0 = None;
    }

    for evt in hover_start.iter() {
        if let Ok(menu_option) = menu_options.get(evt.entity) {
            for (idx, mut cursor) in cursor.iter_mut().enumerate() {
                cursor.0.is_visible = true;

                let offset_x = menu_option.width() / 2. + CURSOR_SIZE.x / 2. + 20.;
                cursor.1.translation.x = offset_x * (idx as f32 * 2. - 1.);

                cursor.1.translation.y = menu_option.offset_y() - 5.;
            }

            active_menu_option.0 = Some(*menu_option);
        }
    }
}

fn mouse_input(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut btns: ResMut<Input<MouseButton>>,
    args: Res<crate::Args>,
    active_menu_option: Res<ActiveMenuOption>,
) {
    if btns.just_pressed(MouseButton::Left)
        && matches!(active_menu_option.0, Some(MenuOption::Draft))
    {
        start_new_game(&mut commands, &mut app_state, &args);

        // required to workaround bug?
        btns.reset(MouseButton::Left);
    }
}

fn animate_cursor(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite), With<Cursor>>,
) {
    for (mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            sprite.index = (sprite.index + 1) % 8;
        }
    }
}
