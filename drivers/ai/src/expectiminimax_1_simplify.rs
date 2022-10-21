use crate::battle_system_probabilities::lookup;
use tetra_master_core as core;

use super::Action;

pub struct Ai(State);

pub fn init(max_depth: usize, player: core::Player, setup: &core::Setup) -> Ai {
    Ai(State::new(max_depth, player, setup))
}

impl super::Ai for Ai {
    fn get_action(&mut self) -> crate::Action {
        match expectiminimax_search(self.0.clone()) {
            Action::PlaceCard(cmd) => crate::Action::PlaceCard(cmd),
            Action::PickBattle(cmd) => crate::Action::PickBattle(cmd),
        }
    }

    fn apply_place_card(&mut self, cmd: core::PlaceCard) {
        self.0.handle_place_card(cmd);
    }

    fn apply_pick_battle(&mut self, cmd: core::PickBattle) {
        self.0.handle_pick_battle(cmd);
    }

    fn apply_resolve_battle(&mut self, cmd: &core::ResolveBattle) {
        self.0.handle_resolve_battle(ResolveBattle::Command(cmd));
    }
}

#[derive(Debug, Clone, Copy)]
struct Resolution {
    winner: core::BattleWinner,
    probability: f32,
}

#[derive(Debug)]
enum ResolveBattle<'a> {
    Command(&'a core::ResolveBattle),
    Manual(core::BattleWinner),
}

//**************************************************************************************************
// expectiminimax logic

fn expectiminimax_search(state: State) -> Action {
    reset!();
    indent!(module_path!());

    debug_assert!(state.player == state.turn);

    // same logic as max_value but also tracks which move has the highest value
    let mut curr_value = f32::NEG_INFINITY;
    let mut selected_action = None;
    for action in state.get_actions() {
        indent!("{action}");
        let new_state_value = state_value(state.apply_action(action));
        dedent!("{action} | {new_state_value}");

        if new_state_value > curr_value {
            curr_value = new_state_value;
            selected_action = Some(action);
        }
    }
    let selected_action = selected_action.unwrap();

    log!("SELECTED {selected_action} | {curr_value}");
    dedent!();

    selected_action
}

#[inline(always)]
fn state_value(state: State) -> f32 {
    if let Status::WaitingResolveBattle(_) = state.status {
        chance_value(state)
    } else if state.turn == state.player {
        max_value(state)
    } else {
        min_value(state)
    }
}

fn max_value(state: State) -> f32 {
    if state.is_terminal() {
        let value = state.utility();
        log!("TERMINAL | {value}");
        return value;
    }

    indent!("MAX");
    let mut curr_value = f32::NEG_INFINITY;
    for action in state.get_actions() {
        indent!("{action}");
        let new_state_value = state_value(state.apply_action(action));
        dedent!("{action} | {new_state_value}");

        curr_value = curr_value.max(new_state_value);
    }
    dedent!("MAX | {curr_value}");
    curr_value
}

fn min_value(state: State) -> f32 {
    if state.is_terminal() {
        let value = state.utility();
        log!("TERMINAL | {value}");
        return value;
    }

    indent!("MIN");
    let mut curr_value = f32::INFINITY;
    for action in state.get_actions() {
        indent!("{action}");
        let new_state_value = state_value(state.apply_action(action));
        dedent!("{action} | {new_state_value}");

        curr_value = curr_value.min(new_state_value);
    }
    dedent!("MIN | {curr_value}");
    curr_value
}

fn chance_value(state: State) -> f32 {
    indent!("CHANCE");
    let mut sum_value = 0.0;
    for resolution in state.get_resolutions() {
        indent!("{resolution:?}");
        let raw_value = state_value(state.apply_resolution(resolution));
        let probability = resolution.probability;
        let value = probability * raw_value;
        dedent!("{resolution:?} | probability({probability}) * value({raw_value}) = {value}");

        sum_value += value;
    }
    dedent!("CHANCE | {sum_value}");
    sum_value
}

//**************************************************************************************************
// game logic

type Hand = [Option<core::Card>; core::HAND_SIZE];
type Board = [Cell; core::BOARD_SIZE];

#[derive(Debug, Clone, Copy, PartialEq)]
struct OwnedCard {
    owner: core::Player,
    card: core::Card,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone)]
struct State {
    max_depth: usize,
    depth: usize,
    player: core::Player,
    status: Status,
    turn: core::Player,
    board: Board,
    hand_blue: Hand,
    hand_red: Hand,
    battle_system: core::BattleSystem,
}

#[derive(Debug, Clone)]
enum Status {
    WaitingPlaceCard,
    WaitingResolveBattle(WaitingResolveBattle),
    WaitingPickBattle {
        attacker_cell: u8,
        choices: core::BoardCells,
    },
    GameOver,
}

#[derive(Debug, Clone)]
struct WaitingResolveBattle {
    attacker: BattlerWaitingResolve,
    defender: BattlerWaitingResolve,
}

#[derive(Debug, Clone, Copy)]
struct BattlerWaitingResolve {
    cell: u8,
    value: u8,
}

impl BattlerWaitingResolve {
    fn resolve(self, battle_system: core::BattleSystem, numbers: &[u8]) -> u8 {
        match battle_system {
            core::BattleSystem::Original => {
                let min = self.value << 4; // range: 00, 10, 20, ..., F0
                let max = min | 0xF; // range: 0F, 1F, 2F, ..., FF

                let stat1 = map_number_to_range(numbers[0], min..=max);
                let stat2 = map_number_to_range(numbers[1], ..=stat1);
                stat1 - stat2
            }
            core::BattleSystem::Dice { .. } => {
                // roll {value} dice and return the sum
                let mut sum = 0;
                for idx in 0..self.value {
                    sum += numbers[idx as usize];
                }
                sum
            }
            core::BattleSystem::Deterministic => self.value,
            core::BattleSystem::Test => numbers[0],
        }
    }
}

impl State {
    fn new(max_depth: usize, player: core::Player, cmd: &core::Setup) -> Self {
        fn convert_hand([a, b, c, d, e]: core::Hand) -> Hand {
            [Some(a), Some(b), Some(c), Some(d), Some(e)]
        }

        let mut board: Board = Default::default();
        for cell in cmd.blocked_cells {
            board[cell as usize] = Cell::Blocked;
        }

        Self {
            max_depth,
            depth: 0,
            player,
            status: Status::WaitingPlaceCard,
            turn: cmd.starting_player,
            board,
            hand_blue: convert_hand(cmd.hand_blue),
            hand_red: convert_hand(cmd.hand_red),
            battle_system: cmd.battle_system,
        }
    }

    fn is_terminal(&self) -> bool {
        self.depth >= self.max_depth || matches!(self.status, Status::GameOver)
    }

    fn utility(&self) -> f32 {
        let mut count = 0.0;
        for cell in self.board {
            if let Cell::Card(card) = cell {
                if card.owner == self.player {
                    count += 1.0;
                } else {
                    count -= 1.0;
                }
            }
        }
        count
    }

    fn get_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        match self.status {
            Status::WaitingPlaceCard => {
                let hand = match self.turn {
                    core::Player::Blue => &self.hand_blue,
                    core::Player::Red => &self.hand_red,
                }
                .iter()
                .enumerate()
                .filter(|(_, card)| card.is_some())
                .map(|(idx, _)| idx);

                let empty_cells = self
                    .board
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| matches!(cell, Cell::Empty))
                    .map(|(idx, _)| idx);

                for cell in empty_cells {
                    for card in hand.clone() {
                        actions.push(Action::PlaceCard(core::PlaceCard {
                            player: self.turn,
                            card: card as u8,
                            cell: cell as u8,
                        }));
                    }
                }
            }
            Status::WaitingPickBattle { choices, .. } => {
                for cell in choices {
                    actions.push(Action::PickBattle(core::PickBattle {
                        player: self.turn,
                        cell,
                    }));
                }
            }
            _ => unreachable!(),
        }
        actions
    }

    fn get_resolutions(&self) -> Vec<Resolution> {
        let mut resolutions = Vec::new();
        match self.status {
            Status::WaitingResolveBattle(WaitingResolveBattle { attacker, defender }) => {
                let attack_win_prob = lookup(self.battle_system, attacker.value, defender.value);

                if attack_win_prob != 0.0 {
                    resolutions.push(Resolution {
                        winner: core::BattleWinner::Attacker,
                        probability: attack_win_prob,
                    });
                }

                if attack_win_prob != 1.0 {
                    resolutions.push(Resolution {
                        winner: core::BattleWinner::Defender,
                        probability: 1.0 - attack_win_prob,
                    });
                }
            }
            _ => unreachable!(),
        }
        resolutions
    }

    fn apply_action(&self, action: Action) -> Self {
        let mut clone = self.clone();

        match action {
            Action::PlaceCard(cmd) => {
                clone.handle_place_card(cmd);
                clone.depth += 1;
            }
            Action::PickBattle(cmd) => clone.handle_pick_battle(cmd),
        }

        clone
    }

    fn apply_resolution(&self, resolution: Resolution) -> Self {
        let mut clone = self.clone();

        clone.handle_resolve_battle(ResolveBattle::Manual(resolution.winner));

        clone
    }

    fn handle_place_card(&mut self, cmd: core::PlaceCard) {
        if let Status::WaitingPlaceCard = self.status {
            let hand = match self.turn {
                core::Player::Blue => &mut self.hand_blue,
                core::Player::Red => &mut self.hand_red,
            };

            // remove the card from the hand
            let card = hand[cmd.card as usize].take().unwrap();

            // place card onto the board
            let owner = self.turn;
            self.board[cmd.cell as usize] = Cell::Card(OwnedCard { owner, card });

            self.resolve_interactions(cmd.cell);
        }
    }

    fn handle_resolve_battle(&mut self, cmd: ResolveBattle) {
        if let Status::WaitingResolveBattle(ref status) = self.status {
            let attacker = status.attacker;
            let defender = status.defender;

            let winner = match cmd {
                ResolveBattle::Manual(winner) => winner,
                ResolveBattle::Command(cmd) => {
                    let attacker_roll = attacker.resolve(self.battle_system, &cmd.attack_roll);
                    let defender_roll = defender.resolve(self.battle_system, &cmd.defend_roll);

                    use std::cmp::Ordering;
                    match attacker_roll.cmp(&defender_roll) {
                        Ordering::Greater => core::BattleWinner::Attacker,
                        Ordering::Less => core::BattleWinner::Defender,
                        Ordering::Equal => core::BattleWinner::None,
                    }
                }
            };

            // flip losing card
            let loser_cell = match winner {
                core::BattleWinner::Defender | core::BattleWinner::None => {
                    self.flip(attacker.cell);
                    attacker.cell
                }
                core::BattleWinner::Attacker => {
                    self.flip(defender.cell);
                    defender.cell
                }
            };

            // combo flip any cards the losing card points at
            for &(comboed_cell, arrow) in get_possible_neighbours(loser_cell) {
                let loser = match &self.board[loser_cell as usize] {
                    Cell::Card(card) => card,
                    _ => unreachable!(),
                };
                let comboed = match &self.board[comboed_cell as usize] {
                    Cell::Card(card) => card,
                    _ => continue,
                };

                if !does_interact(*loser, *comboed, arrow) {
                    continue;
                }

                self.flip(comboed_cell);
            }

            // if the attacker won
            // resolve further interactions
            if winner == core::BattleWinner::Attacker {
                self.resolve_interactions(attacker.cell);
            } else {
                // next turn
                if !self.is_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
        }
    }

    fn handle_pick_battle(&mut self, cmd: core::PickBattle) {
        if let Status::WaitingPickBattle { attacker_cell, .. } = self.status {
            let defender_cell = cmd.cell;

            self.resolve_battle(attacker_cell, defender_cell);
        }
    }

    fn resolve_interactions(&mut self, attacker_cell: u8) {
        let attacker = match self.board[attacker_cell as usize] {
            Cell::Card(card) => card,
            _ => unreachable!(),
        };

        let mut defenders = core::BoardCells::NONE;
        let mut non_defenders = core::BoardCells::NONE;
        for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
            let defender = match self.board[defender_cell as usize] {
                Cell::Card(card) => card,
                _ => continue,
            };

            if !does_interact(attacker, defender, arrow) {
                continue;
            }

            if defender.card.arrows.has_any(arrow.reverse()) {
                defenders.set(defender_cell);
            } else {
                non_defenders.set(defender_cell);
            }
        }

        match defenders.len() {
            0 => {
                // no battles, flip non-defenders
                for cell in non_defenders {
                    self.flip(cell);
                }

                // no more interactions found, next turn
                if !self.is_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
            1 => {
                // handle battle
                let defender_cell = defenders.into_iter().next().unwrap();
                self.resolve_battle(attacker_cell, defender_cell);
            }
            _ => {
                // handle multiple possible battles
                self.status = Status::WaitingPickBattle {
                    attacker_cell,
                    choices: defenders,
                };
            }
        }
    }

    fn flip(&mut self, cell: u8) {
        let card = match &mut self.board[cell as usize] {
            Cell::Card(card) => card,
            _ => unreachable!(),
        };
        card.owner = card.owner.opposite();
    }

    fn resolve_battle(&mut self, attacker_cell: u8, defender_cell: u8) {
        let attacker = match &self.board[attacker_cell as usize] {
            Cell::Card(owned) => owned.card,
            _ => unreachable!(),
        };
        let defender = match &self.board[defender_cell as usize] {
            Cell::Card(owned) => owned.card,
            _ => unreachable!(),
        };

        let attacker_value = get_attack_stat(attacker);
        let defender_value = get_defend_stat(attacker, defender);

        self.status = Status::WaitingResolveBattle(WaitingResolveBattle {
            attacker: BattlerWaitingResolve {
                cell: attacker_cell,
                value: attacker_value,
            },
            defender: BattlerWaitingResolve {
                cell: defender_cell,
                value: defender_value,
            },
        });
    }

    fn is_game_over(&mut self) -> bool {
        if self.hand_blue.iter().all(Option::is_none) && self.hand_red.iter().all(Option::is_none) {
            self.status = Status::GameOver;

            true
        } else {
            self.status = Status::WaitingPlaceCard;

            false
        }
    }
}

fn get_attack_stat(attacker: core::Card) -> u8 {
    if let core::CardType::Assault = attacker.card_type {
        // use the highest stat
        let att = attacker.attack;
        let phy = attacker.physical_defense;
        let mag = attacker.magical_defense;
        if mag > att && mag > phy {
            mag
        } else if phy > att {
            phy
        } else {
            att
        }
    } else {
        // otherwise use the attack stat
        attacker.attack
    }
}

fn get_defend_stat(attacker: core::Card, defender: core::Card) -> u8 {
    match attacker.card_type {
        core::CardType::Physical => defender.physical_defense,
        core::CardType::Magical => defender.magical_defense,
        core::CardType::Exploit => {
            // use the lowest defense stat
            if defender.physical_defense < defender.magical_defense {
                defender.physical_defense
            } else {
                defender.magical_defense
            }
        }
        core::CardType::Assault => {
            // use the lowest stat
            let att = defender.attack;
            let phy = defender.physical_defense;
            let mag = defender.magical_defense;
            if att < phy && att < mag {
                att
            } else if phy < mag {
                phy
            } else {
                mag
            }
        }
    }
}

fn does_interact(
    attacker: OwnedCard,
    defender: OwnedCard,
    arrow_to_defender: core::Arrows,
) -> bool {
    // they don't interact if both cards belong to the same player
    if defender.owner == attacker.owner {
        return false;
    }

    // they interact if the attacking card has an arrow in the direction of the defender
    attacker.card.arrows.has_any(arrow_to_defender)
}

// returns neighbouring cells along with the arrow that points at them
fn get_possible_neighbours(cell: u8) -> &'static [(u8, core::Arrows)] {
    const U: core::Arrows = core::Arrows::UP;
    const UR: core::Arrows = core::Arrows::UP_RIGHT;
    const R: core::Arrows = core::Arrows::RIGHT;
    const DR: core::Arrows = core::Arrows::DOWN_RIGHT;
    const D: core::Arrows = core::Arrows::DOWN;
    const DL: core::Arrows = core::Arrows::DOWN_LEFT;
    const L: core::Arrows = core::Arrows::LEFT;
    const UL: core::Arrows = core::Arrows::UP_LEFT;
    #[rustfmt::skip]
    const LOOKUP: [&[(u8, core::Arrows)]; 16] = [
        &[(0x1, R), (0x4, D), (0x5, DR)],
        &[(0x0, L), (0x2, R), (0x4, DL), (0x5, D), (0x6, DR)],
        &[(0x1, L), (0x3, R), (0x5, DL), (0x6, D), (0x7, DR)],
        &[(0x2, L), (0x6, DL), (0x7, D)],
        &[(0x0, U), (0x1, UR), (0x5, R), (0x8, D), (0x9, DR)],
        &[(0x0, UL), (0x1, U), (0x2, UR), (0x4, L), (0x6, R), (0x8, DL), (0x9, D), (0xA, DR)],
        &[(0x1, UL), (0x2, U), (0x3, UR), (0x5, L), (0x7, R), (0x9, DL), (0xA, D), (0xB, DR)],
        &[(0x3, U), (0xB, D), (0xA, DL), (0x6, L), (0x2, UL)],
        &[(0x4, U), (0x5, UR), (0x9, R), (0xD, DR), (0xC, D)],
        &[(0x5, U), (0x6, UR), (0xA, R), (0xE, DR), (0xD, D), (0xC, DL), (0x8, L), (0x4, UL)],
        &[(0x6, U), (0x7, UR), (0xB, R), (0xF, DR), (0xE, D), (0xD, DL), (0x9, L), (0x5, UL)],
        &[(0x6, UL), (0x7, U), (0xA, L), (0xE, DL), (0xF, D)],
        &[(0x8, U), (0x9, UR), (0xD, R)],
        &[(0x8, UL), (0x9, U), (0xA, UR), (0xC, L), (0xE, R)],
        &[(0x9, UL), (0xA, U), (0xB, UR), (0xD, L), (0xF, R)],
        &[(0xA, UL), (0xB, U), (0xE, L)],
    ];
    LOOKUP[cell as usize]
}

fn map_number_to_range(num: u8, range: impl std::ops::RangeBounds<u8>) -> u8 {
    // Simple way to map the given num to the range 0..max
    // This isn't a perfect mapping but will suffice
    // src: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
    fn map_0_to_max(num: u8, max: u8) -> u8 {
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

    if min == u8::MIN {
        if max == u8::MAX {
            num
        } else {
            map_0_to_max(num, max)
        }
    } else {
        min + map_0_to_max(num, max - min + 1)
    }
}
