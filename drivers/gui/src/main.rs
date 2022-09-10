use bevy::prelude::*;

use tetra_master_core::{Command, Driver, Response};

const RENDER_SIZE: (f32, f32) = (320., 240.);
const SCREEN_SIZE: (f32, f32) = (RENDER_SIZE.0 * 4., RENDER_SIZE.1 * 4.);

// color picked from the background.png file
const CLEAR_COLOR: Color = Color::rgb(0.03137255, 0.03137255, 0.03137255);

enum GameState {
    Setup,
    PickingHandP1 {
        blocked_cells: Vec<u8>,
    },
    PickingHandP2 {
        blocked_cells: Vec<u8>,
    },
    Play {
        hand_p1: Vec<u8>,
        hand_p2: Vec<u8>,
        blocked_cells: Vec<u8>,
        choices: Vec<u8>,
    },
    GameOver,
}

#[derive(Debug, clap::Parser)]
struct Args {
    implementation: String,
}

fn main() {
    let args = {
        use clap::Parser;
        Args::parse()
    };

    let driver = DriverWithLog {
        log: Vec::new(),
        inner: Driver::new(&args.implementation).log(),
    };

    App::new()
        .insert_resource(bevy::render::texture::ImageSettings::default_nearest())
        .insert_resource(WindowDescriptor {
            title: "Tetra Master".to_string(),
            width: SCREEN_SIZE.0,
            height: SCREEN_SIZE.1,
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: RENDER_SIZE.0,
                min_height: RENDER_SIZE.1,
                ..default()
            },
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .insert_resource(GameState::Setup)
        .insert_resource(driver)
        .add_system(step_game_on_click)
        .run();
}

struct DriverWithLog {
    log: Vec<String>,
    inner: Driver,
}

impl DriverWithLog {
    fn send(&mut self, cmd: Command) -> anyhow::Result<Response> {
        self.log.push(format!("TX {cmd:?}"));

        let res = self.inner.send(cmd)?;

        self.log.push(format!("RX {res:?}"));

        Ok(res)
    }
}

#[derive(Component)]
struct GameLog;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("alexandria.ttf");

    commands.insert_resource(ClearColor(CLEAR_COLOR));

    // change projection so that when the window is resized, the game will scale with it while
    // keeping the aspect ratio
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: RENDER_SIZE.1,
            scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(1.),
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("background.png"),
        ..default()
    });

    commands
        .spawn_bundle(
            TextBundle::from_section(
                "",
                TextStyle {
                    font,
                    font_size: 14.0,
                    color: Color::WHITE,
                },
            )
            .with_text_alignment(TextAlignment::TOP_LEFT)
            .with_style(Style {
                align_self: AlignSelf::FlexStart,
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(10.0),
                    left: Val::Px(10.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(GameLog);
}

fn step_game_on_click(
    buttons: Res<Input<MouseButton>>,
    state: ResMut<GameState>,
    mut driver: ResMut<DriverWithLog>,
    mut game_log: Query<&mut Text, With<GameLog>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    // forwards the game automatically
    let state = state.into_inner();
    match state {
        GameState::Setup => {
            // start a random game
            let blocked_cells = if let Response::SetupOk { blocked_cells, .. } = driver
                .send(Command::Setup {
                    rng: None,
                    battle_system: None,
                    blocked_cells: None,
                    hand_candidates: None,
                })
                .unwrap()
            {
                blocked_cells
            } else {
                panic!()
            };

            *state = GameState::PickingHandP1 { blocked_cells };
        }
        GameState::PickingHandP1 { blocked_cells } => {
            // pick the first hand
            let _ = driver.send(Command::PickHand { hand: 0 }).unwrap();
            *state = GameState::PickingHandP2 {
                blocked_cells: blocked_cells.clone(),
            }
        }
        GameState::PickingHandP2 { blocked_cells } => {
            // pick the second hand
            let _ = driver.send(Command::PickHand { hand: 1 }).unwrap();

            let hand_p1 = vec![0, 1, 2, 3, 4];
            let hand_p2 = vec![0, 1, 2, 3, 4];
            *state = GameState::Play {
                blocked_cells: blocked_cells.clone(),
                hand_p1,
                hand_p2,
                choices: vec![],
            };
        }
        GameState::Play {
            blocked_cells,
            hand_p1,
            hand_p2,
            choices,
        } => {
            // as p1 always goes first, if the p2 hand is empty the game is over
            if !hand_p2.is_empty() {
                let cmd = if choices.is_empty() {
                    // no battle choice to be made, so play the next card in the hard
                    let card = if hand_p2.len() > hand_p1.len() {
                        hand_p2.pop().unwrap()
                    } else {
                        hand_p1.pop().unwrap()
                    };
                    // on the next non-empty cell on the board
                    let cell = (0..16).find(|idx| !blocked_cells.contains(idx)).unwrap();
                    blocked_cells.push(cell);
                    Command::PlaceCard { card, cell }
                } else {
                    // pick the first battle out of the choices
                    Command::PickBattle { cell: choices[0] }
                };

                if let Response::PlaceCardOk { pick_battle, .. } = driver.send(cmd).unwrap() {
                    *choices = pick_battle;
                } else {
                    choices.clear();
                }
            } else {
                *state = GameState::GameOver;
            }
        }
        GameState::GameOver => return,
    }

    // update the log on the screen
    let mut game_log = game_log.get_single_mut().unwrap();
    game_log.sections[0].value = driver.log.join("\n");
}
