use crate::{
    self as core, command,
    response::{self, ErrorResponse},
    BattleSystem, BattleWinner, Card, CommandResponse, Error, Player, BOARD_SIZE, HAND_CANDIDATES,
    HAND_SIZE,
};

mod logic;

type Hand = [Option<Card>; HAND_SIZE];
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];
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
 * PickingHands Types
 */

#[derive(Debug, Clone, PartialEq)]
enum PickingHandsStatus {
    P1Picking,
    P2Picking { p1_pick: u8 },
    Complete { p1_pick: u8, p2_pick: u8 },
}

#[derive(Debug, Clone)]
pub struct PickingHandsState {
    status: PickingHandsStatus,
    // pub(crate) for testing
    pub(crate) rng: Rng,
    board: Board,
    hand_candidates: HandCandidates,
    battle_system: BattleSystem,
}

impl PickingHandsState {
    fn is_complete(&self) -> bool {
        matches!(self.status, PickingHandsStatus::Complete { .. })
    }
}

/*****************************************************************************************
 * InGame Types
 */

#[derive(Debug, Clone, PartialEq)]
enum InGameStatus {
    WaitingPlace,
    WaitingBattle {
        attacker_cell: u8,
        choices: Vec<(u8, Card)>,
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
    p1_hand: Hand,
    p2_hand: Hand,
    battle_system: BattleSystem,
}

impl InGameState {
    fn from_pre_game_state(pre_game_state: &mut PickingHandsState) -> Self {
        fn convert_hand([a, b, c, d, e]: HandCandidate) -> Hand {
            [Some(a), Some(b), Some(c), Some(d), Some(e)]
        }

        let status = InGameStatus::WaitingPlace;
        let turn = Player::P1;

        let rng = pre_game_state.rng.clone();
        let board = pre_game_state.board;

        let (p1_pick, p2_pick) = match pre_game_state.status {
            PickingHandsStatus::Complete { p1_pick, p2_pick } => (p1_pick, p2_pick),
            _ => panic!("Cannot get picks from an incomplete PickingHandsState"),
        };
        let p1_hand = convert_hand(pre_game_state.hand_candidates[p1_pick as usize]);
        let p2_hand = convert_hand(pre_game_state.hand_candidates[p2_pick as usize]);

        let battle_system = pre_game_state.battle_system;

        Self {
            status,
            rng,
            turn,
            board,
            p1_hand,
            p2_hand,
            battle_system,
        }
    }

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
    PickingHands(PickingHandsState),
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
            let board = {
                let mut board: Board = Default::default();
                for cell in self.blocked_cells {
                    board[cell as usize] = Cell::Blocked;
                }
                board
            };

            let state = PickingHandsState {
                status: PickingHandsStatus::P1Picking,
                rng: state.rng.take().unwrap(),
                board,
                hand_candidates: self.hand_candidates,
                battle_system: self.battle_system,
            };

            let blocked_cells = state
                .board
                .iter()
                .enumerate()
                .filter_map(|(idx, cell)| {
                    if let Cell::Blocked = cell {
                        Some(idx as u8)
                    } else {
                        None
                    }
                })
                .collect();
            let battle_system = state.battle_system;
            let hand_candidates = state.hand_candidates;

            *global = GlobalState::PickingHands(state);

            Ok(response::SetupOk {
                blocked_cells,
                battle_system,
                hand_candidates,
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
            GlobalState::PickingHands(PickingHandsState { rng, .. }) => rng,
            GlobalState::InGame(InGameState { rng, .. }) => rng,
        };

        rng.push_numbers(&self.numbers);
        let numbers_left = rng.numbers.len();
        Ok(response::PushRngNumbersOk { numbers_left })
    }
}

impl Step for command::PickHand {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::PickingHands(ref mut state) = global {
            let res = match logic::pre_game_next(state, self) {
                Err(err) => Err(err),
                _ => Ok(response::PickHandOk),
            };

            if state.is_complete() {
                let state = InGameState::from_pre_game_state(state);
                *global = GlobalState::InGame(state);
            }

            res
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PlaceCard {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::InGame(ref mut state) = global {
            let input = InGameInput::Place(self);

            let mut events = Vec::new();
            logic::game_next(state, &mut events, input)?;

            let choices = if let InGameStatus::WaitingBattle { choices, .. } = &state.status {
                choices.as_slice()
            } else {
                &[]
            };

            Ok(response::PlayOk {
                pick_battle: choices.iter().map(|(cell, _)| *cell).collect(),
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
            let input = InGameInput::Battle(self);

            let mut events = Vec::new();
            logic::game_next(state, &mut events, input)?;

            let choices = if let InGameStatus::WaitingBattle { choices, .. } = &state.status {
                choices.as_slice()
            } else {
                &[]
            };

            Ok(response::PlayOk {
                pick_battle: choices.iter().map(|(cell, _)| *cell).collect(),
                events,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}
