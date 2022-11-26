use crate::metrics::Metrics;
use crate::win_probabilities;
use tetra_master_core as core;

pub struct AI {
    con: ConstantState,
    var: VariableState,
}

pub fn init(max_depth: usize, prob_cutoff: f32, player: core::Player, setup: &core::Setup) -> AI {
    AI {
        con: ConstantState::new(max_depth, prob_cutoff, player, setup),
        var: VariableState::new(setup),
    }
}

impl super::AIInterface for AI {
    fn get_action(&mut self) -> crate::Action {
        let player = self.con.player;
        match expectiminimax_search(&mut self.con, self.var.clone()) {
            Action::PlaceCard { cell, card } => {
                crate::Action::PlaceCard(core::PlaceCard { player, cell, card })
            }
            Action::PickBattle { cell } => {
                crate::Action::PickBattle(core::PickBattle { player, cell })
            }
        }
    }

    fn apply_place_card(&mut self, cmd: core::PlaceCard) {
        self.var.handle_place_card(&self.con, cmd.cell, cmd.card);
    }

    fn apply_pick_battle(&mut self, cmd: core::PickBattle) {
        self.var.handle_pick_battle(&self.con, cmd.cell);
    }

    fn apply_resolve_battle(&mut self, cmd: &core::ResolveBattle) {
        self.var.handle_resolve_battle_via_command(&self.con, cmd);
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    PlaceCard { cell: u8, card: u8 },
    PickBattle { cell: u8 },
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::PlaceCard { card, cell, .. } => {
                write!(f, "Place({card:X}, {cell:X})")
            }
            Action::PickBattle { cell, .. } => {
                write!(f, "Pick({cell:X})    ")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Resolution {
    winner: core::BattleWinner,
    probability: f32,
}

//**************************************************************************************************
// expectiminimax logic

fn expectiminimax_search(con: &mut ConstantState, var: VariableState) -> Action {
    reset!();
    indent!(module_path!());

    debug_assert!(con.player == var.turn);

    // same logic as max_value but also tracks which move has the highest value
    let (mut alpha, beta) = (f32::NEG_INFINITY, f32::INFINITY);
    let mut curr_value = f32::NEG_INFINITY;
    let mut selected_action = None;
    for action in var.get_actions() {
        indent!("{action}");
        let new_state_value = state_value(con, var.apply_action(con, action), alpha, beta);
        dedent!("{action} | {new_state_value}");

        if new_state_value > curr_value {
            curr_value = new_state_value;
            alpha = curr_value.max(alpha);
            selected_action = Some(action);
        }
    }
    let selected_action = selected_action.unwrap();

    log!("SELECTED {selected_action} | {curr_value}");
    dedent!();

    con.metrics.print_report();

    selected_action
}

#[inline(always)]
fn state_value(con: &mut ConstantState, var: VariableState, alpha: f32, beta: f32) -> f32 {
    con.metrics.inc_expanded_nodes();

    match &var.status {
        Status::WaitingResolveBattle(_) => chance_value(con, var, alpha, beta),
        Status::WaitingPlaceCard { .. } => -negamax_value(con, var, -beta, -alpha),
        Status::WaitingPickBattle { .. } => negamax_value(con, var, alpha, beta),
        Status::GameOver => {
            con.metrics.inc_terminal_leafs();

            let value = var.evaluate();
            log!("TERMINAL | {value}");
            value
        }
    }
}

fn negamax_value(con: &mut ConstantState, var: VariableState, mut alpha: f32, beta: f32) -> f32 {
    if var.depth >= con.max_depth || var.status.is_game_over() {
        con.metrics.inc_depth_limit_leafs();

        let value = var.evaluate();
        log!("DEPTH-LIMIT | {value}");
        return value;
    }

    indent!("NEGAMAX alpha({alpha}) beta({beta})");
    let mut curr_value = f32::NEG_INFINITY;
    for action in var.get_actions() {
        indent!("{action}");
        let new_state_value = state_value(con, var.apply_action(con, action), alpha, beta);
        dedent!("{action} | {new_state_value}");

        if new_state_value > curr_value {
            curr_value = new_state_value;
            alpha = curr_value.max(alpha);
        }
        if alpha >= beta {
            con.metrics.inc_pruned_nodes(var.depth as usize);

            log!("PRUNE | alpha({alpha}) >= beta({beta})");
            break;
        }
    }
    dedent!("NEGAMAX | {curr_value}");
    curr_value
}

fn chance_value(con: &mut ConstantState, var: VariableState, mut alpha: f32, mut beta: f32) -> f32 {
    let resolutions = var.get_resolutions(con);

    // Reset the alpha-beta values if we hit a chance node with multiple children to avoid
    // over-pruning.
    //
    // As chance nodes require all of their children to have accurate values, pruning any child
    // nodes (based on nodes above) risks affecting the final value of the chance node.
    //
    // As chance nodes with only one child can only have one possible value, there is no risk of
    // over-pruning.
    if resolutions.len() > 1 {
        (alpha, beta) = (f32::NEG_INFINITY, f32::INFINITY);
    }

    indent!("CHANCE alpha({alpha}) beta({beta})");
    let mut sum_value = 0.0;
    for resolution in resolutions {
        indent!("{resolution:?}");
        let raw_value = state_value(con, var.apply_resolution(con, resolution), alpha, beta);
        let probability = resolution.probability;
        let value = probability * raw_value;
        dedent!("{resolution:?} | probability({probability}) * value({raw_value}) = {value}");

        sum_value += value;
    }
    dedent!("CHANCE | {sum_value}");
    sum_value
}

//**************************************************************************************************
// state

// Game state which remains constant
#[derive(Debug)]
struct ConstantState {
    metrics: Metrics,
    max_depth: u8,
    prob_cutoff: f32,
    player: core::Player,
    battle_system: core::BattleSystem,
    cards: [core::Card; 10],
}

// Game state which is variable
#[derive(Debug, Clone)]
struct VariableState {
    depth: u8,
    status: Status,
    turn: core::Player,
    board: Board,
    hand_blue: Hand,
    hand_red: Hand,
}

impl ConstantState {
    fn new(max_depth: usize, prob_cutoff: f32, player: core::Player, cmd: &core::Setup) -> Self {
        Self {
            metrics: Metrics::new(module_path!()),
            max_depth: max_depth as u8,
            prob_cutoff,
            player,
            battle_system: cmd.battle_system,
            cards: [
                cmd.hand_blue[0],
                cmd.hand_blue[1],
                cmd.hand_blue[2],
                cmd.hand_blue[3],
                cmd.hand_blue[4],
                cmd.hand_red[0],
                cmd.hand_red[1],
                cmd.hand_red[2],
                cmd.hand_red[3],
                cmd.hand_red[4],
            ],
        }
    }

    fn get_card(&self, card: u8) -> core::Card {
        self.cards[card as usize]
    }
}

impl VariableState {
    fn new(cmd: &core::Setup) -> Self {
        let mut board: Board = Default::default();
        for cell in cmd.blocked_cells {
            board[cell as usize] = Cell::blocked();
        }

        Self {
            depth: 0,
            status: Status::WaitingPlaceCard,
            turn: cmd.starting_player,
            board,
            hand_blue: Hand::new(),
            hand_red: Hand::new(),
        }
    }
}

//**************************************************************************************************
// game logic

type Board = [Cell; core::BOARD_SIZE];

// Bitset where set bits indicate the card has not been placed
#[derive(Debug, Clone, Copy)]
struct Hand(u8);

impl Hand {
    fn new() -> Self {
        Self(0b0001_1111)
    }

    #[inline(always)]
    fn is_set(&self, idx: u8) -> bool {
        debug_assert!(idx < 5);
        self.0 & (1 << idx) != 0
    }

    #[inline(always)]
    fn unset(&mut self, idx: u8) {
        debug_assert!(idx < 5);
        self.0 ^= 1 << idx;
    }

    #[inline(always)]
    fn is_empty(self) -> bool {
        self.0 == 0
    }
}

// A bit packed enum that represents Cell::Empty, Cell::Blocked, or Cell::Card(owner, card_idx)
// the high 4 bits is the card_idx (index into the ConstantState.cards array)
// the low 4 bits are
//     0b0001 => Card(Blue, _)
//     0b0010 => Card(Red, _)
//     0b0100 => Blocked
//     0b1000 => Empty
#[derive(Clone, Copy, PartialEq)]
struct Cell(u8);

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cell({:04b}_{:04b})", self.0 >> 4, self.0 & 0b1111)
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::empty()
    }
}

impl Cell {
    const BLUE: u8 = 0b0001;
    const RED: u8 = 0b0010;
    const BLOCKED: u8 = 0b0100;
    const EMPTY: u8 = 0b1000;

    fn empty() -> Self {
        Self(Self::EMPTY)
    }

    fn blocked() -> Self {
        Self(Self::BLOCKED)
    }

    fn card(owner: core::Player, card_idx: u8) -> Self {
        let high = card_idx << 4;
        let low = match owner {
            core::Player::Blue => Self::BLUE,
            core::Player::Red => Self::RED,
        };
        Self(high | low)
    }

    #[inline(always)]
    fn is_empty(self) -> bool {
        self.0 & Self::EMPTY != 0
    }

    #[inline(always)]
    fn is_card(self) -> bool {
        self.0 & (Self::BLUE | Self::RED) != 0
    }

    #[inline(always)]
    fn flip_card(&mut self) {
        debug_assert!(self.is_card());
        self.0 ^= Self::BLUE | Self::RED;
    }

    #[inline(always)]
    fn to_card_owner(self) -> core::Player {
        debug_assert!(self.is_card());
        match self.0 & 0b1111 {
            Self::BLUE => core::Player::Blue,
            Self::RED => core::Player::Red,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn to_card_idx(self) -> u8 {
        debug_assert!(self.is_card());
        self.0 >> 4
    }

    fn to_card(self) -> OwnedCard {
        debug_assert!(self.is_card());
        match self.0 & 0b1111 {
            Self::BLUE => OwnedCard {
                owner: core::Player::Blue,
                card: self.0 >> 4,
            },
            Self::RED => OwnedCard {
                owner: core::Player::Red,
                card: self.0 >> 4,
            },
            _ => unreachable!(),
        }
    }

    fn try_to_card(self) -> Option<OwnedCard> {
        match self.0 & 0b1111 {
            Self::BLUE => Some(OwnedCard {
                owner: core::Player::Blue,
                card: self.0 >> 4,
            }),
            Self::RED => Some(OwnedCard {
                owner: core::Player::Red,
                card: self.0 >> 4,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct OwnedCard {
    owner: core::Player,
    card: u8,
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

impl Status {
    fn is_game_over(&self) -> bool {
        matches!(self, Status::GameOver)
    }
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

impl VariableState {
    fn evaluate(&self) -> f32 {
        let mut count = 0.0;
        for cell in self.board {
            if cell.is_card() {
                if cell.to_card_owner() == self.turn {
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
                    core::Player::Blue => self.hand_blue,
                    core::Player::Red => self.hand_red,
                };

                let empty_cells = self
                    .board
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| cell.is_empty())
                    .map(|(idx, _)| idx as u8);

                for cell in empty_cells {
                    for card in 0..5 {
                        if hand.is_set(card) {
                            actions.push(Action::PlaceCard { card, cell });
                        }
                    }
                }
            }
            Status::WaitingPickBattle { choices, .. } => {
                for cell in choices {
                    actions.push(Action::PickBattle { cell });
                }
            }
            _ => unreachable!(),
        }
        actions
    }

    fn get_resolutions(&self, con: &ConstantState) -> arrayvec::ArrayVec<Resolution, 2> {
        let mut resolutions = arrayvec::ArrayVec::new();
        match self.status {
            Status::WaitingResolveBattle(WaitingResolveBattle { attacker, defender }) => {
                let mut attack_win_prob =
                    win_probabilities::lookup(con.battle_system, attacker.value, defender.value);

                if attack_win_prob < con.prob_cutoff {
                    attack_win_prob = 0.0;
                }
                if attack_win_prob > (1.0 - con.prob_cutoff) {
                    attack_win_prob = 1.0;
                }

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

    fn apply_action(&self, con: &ConstantState, action: Action) -> Self {
        let mut clone = self.clone();

        match action {
            Action::PlaceCard { cell, card } => {
                clone.handle_place_card(con, cell, card);
                clone.depth += 1;
            }
            Action::PickBattle { cell } => clone.handle_pick_battle(con, cell),
        }

        clone
    }

    fn apply_resolution(&self, con: &ConstantState, resolution: Resolution) -> Self {
        let mut clone = self.clone();

        clone.handle_resolve_battle(con, resolution.winner);

        clone
    }

    fn handle_place_card(&mut self, con: &ConstantState, cell: u8, card: u8) {
        if let Status::WaitingPlaceCard = self.status {
            let (hand, card_idx) = match self.turn {
                core::Player::Blue => (&mut self.hand_blue, card),
                core::Player::Red => (&mut self.hand_red, card + 5),
            };

            // remove the card from the hand
            hand.unset(card);

            // place card onto the board
            self.board[cell as usize] = Cell::card(self.turn, card_idx);

            self.resolve_interactions(con, cell);
        }
    }

    fn handle_resolve_battle_via_command(
        &mut self,
        con: &ConstantState,
        cmd: &core::ResolveBattle,
    ) {
        if let Status::WaitingResolveBattle(ref status) = self.status {
            let attacker = status.attacker;
            let defender = status.defender;

            let attacker_roll = attacker.resolve(con.battle_system, &cmd.attack_roll);
            let defender_roll = defender.resolve(con.battle_system, &cmd.defend_roll);

            use std::cmp::Ordering;
            let winner = match attacker_roll.cmp(&defender_roll) {
                Ordering::Greater => core::BattleWinner::Attacker,
                Ordering::Less => core::BattleWinner::Defender,
                Ordering::Equal => core::BattleWinner::None,
            };

            self.handle_resolve_battle(con, winner)
        }
    }

    fn handle_resolve_battle(&mut self, con: &ConstantState, winner: core::BattleWinner) {
        if let Status::WaitingResolveBattle(ref status) = self.status {
            let attacker = status.attacker;
            let defender = status.defender;

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
            let loser = self.board[loser_cell as usize].to_card();
            for &(comboed_cell, arrow) in get_possible_neighbours(loser_cell) {
                let comboed = match self.board[comboed_cell as usize].try_to_card() {
                    Some(card) => card,
                    _ => continue,
                };

                if !does_interact(con, loser, comboed, arrow) {
                    continue;
                }

                self.flip(comboed_cell);
            }

            // if the attacker won
            // resolve further interactions
            if winner == core::BattleWinner::Attacker {
                self.resolve_interactions(con, attacker.cell);
            } else {
                // next turn
                if !self.check_for_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
        }
    }

    fn handle_pick_battle(&mut self, con: &ConstantState, defender_cell: u8) {
        if let Status::WaitingPickBattle { attacker_cell, .. } = self.status {
            self.resolve_battle(con, attacker_cell, defender_cell);
        }
    }

    fn resolve_interactions(&mut self, con: &ConstantState, attacker_cell: u8) {
        let attacker = self.board[attacker_cell as usize].to_card();

        let mut defenders = core::BoardCells::NONE;
        let mut non_defenders = core::BoardCells::NONE;
        for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
            let defender = match self.board[defender_cell as usize].try_to_card() {
                Some(card) => card,
                _ => continue,
            };

            if !does_interact(con, attacker, defender, arrow) {
                continue;
            }

            if con.get_card(defender.card).arrows.has_any(arrow.reverse()) {
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
                if !self.check_for_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
            1 => {
                // handle battle
                let defender_cell = defenders.into_iter().next().unwrap();
                self.resolve_battle(con, attacker_cell, defender_cell);
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
        self.board[cell as usize].flip_card();
    }

    fn resolve_battle(&mut self, con: &ConstantState, attacker_cell: u8, defender_cell: u8) {
        let attacker = con.get_card(self.board[attacker_cell as usize].to_card_idx());
        let defender = con.get_card(self.board[defender_cell as usize].to_card_idx());

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

    fn check_for_game_over(&mut self) -> bool {
        if self.hand_blue.is_empty() && self.hand_red.is_empty() {
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
    con: &ConstantState,
    attacker: OwnedCard,
    defender: OwnedCard,
    arrow_to_defender: core::Arrows,
) -> bool {
    // they don't interact if both cards belong to the same player
    if defender.owner == attacker.owner {
        return false;
    }

    // they interact if the attacking card has an arrow in the direction of the defender
    let attacker = con.get_card(attacker.card);
    attacker.arrows.has_any(arrow_to_defender)
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
