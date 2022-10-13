use crate::{
    self as core, command,
    response::{self, ErrorResponse},
    BattleSystem, BattleWinner, BoardCells, Card, CommandResponse, Error, Player, BOARD_SIZE,
    HAND_SIZE,
};

mod logic;

type Hand = [Option<Card>; HAND_SIZE];
type Board = [Cell; BOARD_SIZE];

#[derive(Debug, Clone, Copy)]
struct OwnedCard {
    owner: Player,
    card: Card,
}

#[derive(Debug, Clone, Copy)]
enum Cell {
    Blocked,
    Card(OwnedCard),
    Empty,
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

// Rng that relies on numbers been pre-fed into it
#[derive(Debug, Clone, Default)]
pub(crate) struct Rng {
    // pub(crate) for testing
    pub(crate) numbers: std::collections::VecDeque<u8>,
}

impl Rng {
    fn push_numbers(&mut self, numbers: &[u8]) {
        self.numbers.extend(numbers.iter())
    }

    fn gen_u8(&mut self, range: impl std::ops::RangeBounds<u8>) -> u8 {
        // Simple way to map the given num to the range 0..max
        // This isn't a perfect mapping but will suffice
        // src: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
        fn bound(num: u8, max: u8) -> u8 {
            ((num as u16 * max as u16) >> 8) as u8
        }

        use std::ops::Bound::*;

        let min = match range.start_bound() {
            Included(x) => *x,
            Excluded(x) => *x + 1,
            Unbounded => u8::MIN,
        };
        let max = match range.end_bound() {
            Included(x) => *x,
            Excluded(x) => *x - 1,
            Unbounded => u8::MAX,
        };
        debug_assert!(min <= max);

        let num = self
            .numbers
            .pop_front()
            .expect("Ran out of external random numbers");

        if min == u8::MIN {
            if max == u8::MAX {
                num
            } else {
                bound(num, max)
            }
        } else {
            min + bound(num, max - min + 1)
        }
    }
}

/*****************************************************************************************
 * PreSetup Types
 */

#[derive(Debug)]
pub struct PreSetupState {
    // pub(crate) for testing
    pub(crate) rng: Option<Rng>,
}

/*****************************************************************************************
 * InGame Types
 */

#[derive(Debug, Clone, PartialEq)]
enum InGameStatus {
    WaitingPlace,
    WaitingBattle {
        attacker_cell: u8,
        choices: BoardCells,
    },
    GameOver {
        winner: Option<Player>,
    },
}

#[derive(Debug, Clone)]
pub struct InGameState {
    status: InGameStatus,
    // pub(crate) for testing
    pub(crate) rng: Rng,
    turn: Player,
    board: Board,
    hand_blue: Hand,
    hand_red: Hand,
    battle_system: BattleSystem,
}

impl InGameState {
    // take out card from the given cell
    // panics if there is no card in the given cell
    fn take_card(&mut self, cell: u8) -> OwnedCard {
        match std::mem::take(&mut self.board[cell as usize]) {
            Cell::Card(card) => card,
            _ => panic!("Cell didn't have a card"),
        }
    }
}

#[derive(Debug)]
enum InGameInput {
    Place(command::PlaceCard),
    Battle(command::PickBattle),
}

/*****************************************************************************************
 * ReferenceImplementation
 */

pub type ReferenceImplementation = GlobalState;

#[derive(Debug)]
pub enum GlobalState {
    PreSetup(PreSetupState),
    InGame(InGameState),
}

impl GlobalState {
    pub fn new() -> Self {
        GlobalState::PreSetup(PreSetupState {
            rng: Some(Rng::default()),
        })
    }

    pub fn step<C: Step>(&mut self, cmd: C) -> core::Result<C::Response> {
        match cmd.step(self) {
            Err(err) => Err(Error::ErrorResponse(err)),
            Ok(ok) => Ok(ok),
        }
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Step: CommandResponse {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse>;
}

impl Step for command::Setup {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::PreSetup(ref mut state) = global {
            fn convert_hand([a, b, c, d, e]: core::Hand) -> Hand {
                [Some(a), Some(b), Some(c), Some(d), Some(e)]
            }

            let board = {
                let mut board: Board = Default::default();
                for cell in self.blocked_cells {
                    board[cell as usize] = Cell::Blocked;
                }
                board
            };

            let state = InGameState {
                status: InGameStatus::WaitingPlace,
                rng: state.rng.take().unwrap(),
                turn: self.starting_player,
                board,
                hand_blue: convert_hand(self.hand_blue),
                hand_red: convert_hand(self.hand_red),
                battle_system: self.battle_system,
            };

            *global = GlobalState::InGame(state);

            Ok(response::SetupOk {
                blocked_cells: self.blocked_cells,
                battle_system: self.battle_system,
                hand_blue: self.hand_blue,
                hand_red: self.hand_red,
                starting_player: self.starting_player,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PushRngNumbers {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        let rng = match global {
            GlobalState::PreSetup(PreSetupState { rng }) => rng.as_mut().unwrap(),
            GlobalState::InGame(InGameState { rng, .. }) => rng,
        };

        rng.push_numbers(&self.numbers);
        let numbers_left = rng.numbers.len();
        Ok(response::PushRngNumbersOk { numbers_left })
    }
}

impl Step for command::PlaceCard {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::InGame(ref mut state) = global {
            assert!(
                self.player == state.turn,
                "Unexpected player ({}) played move, expected move by {}",
                self.player,
                state.turn,
            );

            let input = InGameInput::Place(self);

            let mut events = Vec::new();
            logic::game_next(state, &mut events, input)?;

            let pick_battle = if let InGameStatus::WaitingBattle { choices, .. } = &state.status {
                *choices
            } else {
                BoardCells::NONE
            };

            Ok(response::PlayOk {
                pick_battle,
                events,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PickBattle {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::InGame(ref mut state) = global {
            assert!(
                self.player == state.turn,
                "Unexpected player ({}) played move, expected move by {}",
                self.player,
                state.turn,
            );

            let input = InGameInput::Battle(self);

            let mut events = Vec::new();
            logic::game_next(state, &mut events, input)?;

            let pick_battle = if let InGameStatus::WaitingBattle { choices, .. } = &state.status {
                *choices
            } else {
                BoardCells::NONE
            };

            Ok(response::PlayOk {
                pick_battle,
                events,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}
