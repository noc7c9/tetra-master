use crate::{
    self as core, command,
    response::{self, ErrorResponse},
    Arrows, BattleSystem, BattleWinner, Card, CardType, CommandResponse, Error, Player, BOARD_SIZE,
    HAND_CANDIDATES, HAND_SIZE, MAX_NUMBER_OF_BLOCKS,
};

mod logic;
mod rng;

use rng::Rng;

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
    rng: Rng,
    board: Board,
    hand_candidates: HandCandidates,
}

impl PickingHandsState {
    fn builder() -> PickingHandsStateBuilder {
        PickingHandsStateBuilder {
            rng: None,
            board: None,
            hand_candidates: None,
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.status, PickingHandsStatus::Complete { .. })
    }
}

#[derive(Debug)]
struct PickingHandsStateBuilder {
    rng: Option<Rng>,
    board: Option<Board>,
    hand_candidates: Option<HandCandidates>,
}

impl PickingHandsStateBuilder {
    fn rng(mut self, rng: Option<Rng>) -> Self {
        self.rng = rng;
        self
    }

    fn board(mut self, board: Option<Board>) -> Self {
        self.board = board;
        self
    }

    fn hand_candidates(mut self, hand_candidates: Option<HandCandidates>) -> Self {
        self.hand_candidates = hand_candidates;
        self
    }

    fn build(self) -> PickingHandsState {
        let status = PickingHandsStatus::P1Picking;
        let mut rng = self.rng.unwrap_or_else(Rng::new);
        let board = self.board.unwrap_or_else(|| rng::random_board(&mut rng));
        let hand_candidates = self
            .hand_candidates
            .unwrap_or_else(|| rng::random_hand_candidates(&mut rng));

        PickingHandsState {
            status,
            rng,
            board,
            hand_candidates,
        }
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
    rng: Rng,
    turn: Player,
    board: Board,
    p1_hand: Hand,
    p2_hand: Hand,
    battle_system: BattleSystem,
}

impl InGameState {
    fn from_pre_game_state(
        pre_game_state: &mut PickingHandsState,
        battle_system: BattleSystem,
    ) -> Self {
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
    PreSetup,
    PickingHands(PickingHandsState, BattleSystem),
    InGame(InGameState),
}

impl GlobalState {
    pub fn new() -> Self {
        GlobalState::PreSetup
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
        if let GlobalState::PreSetup = global {
            let battle_system = self.battle_system.unwrap_or(core::BattleSystem::Original);

            let state = PickingHandsState::builder()
                .rng(self.rng.map(|rng| match rng {
                    core::Rng::Seeded { seed } => Rng::with_seed(seed),
                    // FIXME: remove conversion, why is it a VecDeque?
                    core::Rng::External { rolls } => Rng::new_external(rolls.into_iter().collect()),
                }))
                .hand_candidates(self.hand_candidates)
                .board(self.blocked_cells.map(|blocked_cells| {
                    let mut board: Board = Default::default();
                    for cell in blocked_cells {
                        board[cell as usize] = Cell::Blocked;
                    }
                    board
                }))
                .build();

            let seed = state.rng.initial_seed();
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
            let hand_candidates = state.hand_candidates;

            *global = GlobalState::PickingHands(state, battle_system);

            Ok(response::SetupOk {
                seed,
                battle_system,
                blocked_cells,
                hand_candidates,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PickHand {
    fn step(self, global: &mut GlobalState) -> Result<Self::Response, ErrorResponse> {
        if let GlobalState::PickingHands(ref mut state, battle_system) = global {
            let res = match logic::pre_game_next(state, self) {
                Err(err) => Err(err),
                _ => Ok(response::PickHandOk),
            };

            if state.is_complete() {
                let state = InGameState::from_pre_game_state(state, *battle_system);
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

            Ok(response::PlaceCardOk {
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

            Ok(response::PlaceCardOk {
                pick_battle: choices.iter().map(|(cell, _)| *cell).collect(),
                events,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}
