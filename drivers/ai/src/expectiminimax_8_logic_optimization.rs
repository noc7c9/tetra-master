use crate::interactions;
use crate::metrics::Metrics;
use crate::win_probabilities;
use tetra_master_core as core;

pub struct Ai {
    prealloc: Vec<Vec<Action>>,
    con: ConstantState,
    var: VariableState,
}

pub fn init(max_depth: usize, prob_cutoff: f32, player: core::Player, setup: &core::Setup) -> Ai {
    Ai {
        prealloc: prealloc(max_depth, setup),
        con: ConstantState::new(max_depth as u8, prob_cutoff, player, setup),
        var: VariableState::new(setup),
    }
}

impl Ai {
    pub fn reinit(&mut self, player: core::Player, setup: &core::Setup) {
        let max_depth = self.con.max_depth;
        let prob_cutoff = self.con.prob_cutoff;
        self.con = ConstantState::new(max_depth, prob_cutoff, player, setup);
        self.var = VariableState::new(setup);
    }
}

impl super::Ai for Ai {
    fn get_action(&mut self) -> crate::Action {
        let player = self.con.player;
        match expectiminimax_search(&mut self.prealloc, &mut self.con, self.var.clone()) {
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
        self.var.handle_pick_battle(cmd.cell);
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

fn expectiminimax_search(
    prealloc: &mut [Vec<Action>],
    con: &mut ConstantState,
    var: VariableState,
) -> Action {
    reset!();
    indent!(module_path!());

    debug_assert!(con.player == var.turn);

    // same logic as max_value but also tracks which move has the highest value
    let (mut alpha, beta) = (f32::NEG_INFINITY, f32::INFINITY);
    let mut curr_value = f32::NEG_INFINITY;

    macro_rules! select_action {
        ($prealloc:expr, $actions_iter:expr) => {{
            let mut selected_action = None;
            for action in $actions_iter {
                indent!("{action}");
                let new_state_value =
                    state_value($prealloc, con, var.apply_action(con, action), alpha, beta);
                dedent!("{action} | {new_state_value}");

                if new_state_value > curr_value {
                    curr_value = new_state_value;
                    alpha = curr_value.max(alpha);
                    selected_action = Some(action);
                }
            }
            selected_action.unwrap()
        }};
    }

    let selected_action = match &var.status {
        Status::WaitingPlaceCard => {
            let (head, rest) = prealloc.split_at_mut(1);
            select_action!(rest, var.get_place_card_actions(con, &mut head[0]))
        }
        Status::WaitingPickBattle { .. } => select_action!(prealloc, var.get_pick_battle_actions()),
        _ => unreachable!(),
    };

    log!("SELECTED {selected_action} | {curr_value}");
    dedent!();

    con.metrics.print_report();

    selected_action
}

#[inline(always)]
fn state_value(
    prealloc: &mut [Vec<Action>],
    con: &mut ConstantState,
    var: VariableState,
    alpha: f32,
    beta: f32,
) -> f32 {
    con.metrics.inc_expanded_nodes();

    match &var.status {
        Status::WaitingResolveBattle(_) => chance_value(prealloc, con, var, alpha, beta),
        Status::WaitingPlaceCard { .. } => -negamax_value(prealloc, con, var, -beta, -alpha),
        Status::WaitingPickBattle { .. } => negamax_value(prealloc, con, var, alpha, beta),
        Status::GameOver => {
            con.metrics.inc_terminal_leafs();

            let value = var.evaluate();
            log!("TERMINAL | {value}");
            value
        }
    }
}

fn negamax_value(
    prealloc: &mut [Vec<Action>],
    con: &mut ConstantState,
    var: VariableState,
    mut alpha: f32,
    beta: f32,
) -> f32 {
    if var.depth >= con.max_depth || var.status.is_game_over() {
        con.metrics.inc_depth_limit_leafs();

        let value = var.evaluate();
        log!("DEPTH-LIMIT | {value}");
        return value;
    }

    macro_rules! negamax_value {
        ($prealloc:expr, $actions_iter:expr) => {{
            indent!("NEGAMAX alpha({alpha}) beta({beta})");
            let mut curr_value = f32::NEG_INFINITY;
            for action in $actions_iter {
                indent!("{action}");
                let new_state_value =
                    state_value($prealloc, con, var.apply_action(con, action), alpha, beta);
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
        }};
    }

    match &var.status {
        Status::WaitingPlaceCard => {
            let (head, rest) = prealloc.split_at_mut(1);
            negamax_value!(rest, var.get_place_card_actions(con, &mut head[0]))
        }
        Status::WaitingPickBattle { .. } => negamax_value!(prealloc, var.get_pick_battle_actions()),
        _ => unreachable!(),
    }
}

fn chance_value(
    prealloc: &mut [Vec<Action>],
    con: &mut ConstantState,
    var: VariableState,
    mut alpha: f32,
    mut beta: f32,
) -> f32 {
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
        let new_var = var.apply_resolution(con, resolution);
        let raw_value = state_value(prealloc, con, new_var, alpha, beta);
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

const NUM_CARDS: usize = core::HAND_SIZE * 2;

// Game state which remains constant
#[derive(Debug)]
struct ConstantState {
    metrics: Metrics,
    max_depth: u8,
    prob_cutoff: f32,
    player: core::Player,
    battle_system: core::BattleSystem,
    cells_blocked: core::BoardCells,
    // all interactions for each card in the game
    interactions: [[u16; core::BOARD_SIZE]; NUM_CARDS],
    // all the matchups between each pair of cards in the game
    matchups: [[Matchup; NUM_CARDS]; NUM_CARDS],
}

// Game state which is variable
#[derive(Debug, Clone)]
struct VariableState {
    depth: u8,
    status: Status,
    turn: core::Player,
    board: Board,
    cells_blue: u16,
    cells_red: u16,
    hand_blue: Hand,
    hand_red: Hand,
}

fn prealloc(max_depth: usize, cmd: &core::Setup) -> Vec<Vec<Action>> {
    let hand_size = core::HAND_SIZE;
    let board_size = core::BOARD_SIZE - cmd.blocked_cells.len();
    let max_moves = hand_size * board_size;

    let mut vecs = Vec::with_capacity(max_depth);
    for _ in 0..max_depth {
        vecs.push(Vec::with_capacity(max_moves));
    }
    vecs
}

impl ConstantState {
    fn new(max_depth: u8, prob_cutoff: f32, player: core::Player, cmd: &core::Setup) -> Self {
        // this is the order of card indexes, blue cards then red cards
        let cards = [
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
        ];

        let mut interactions = Vec::with_capacity(NUM_CARDS);
        for card in cards {
            let mut card_interactions = Vec::with_capacity(core::BOARD_SIZE);
            for cell in 0..core::BOARD_SIZE {
                card_interactions.push(interactions::lookup(card.arrows, cell as u8));
            }
            interactions.push(card_interactions.try_into().unwrap());
        }
        let interactions = interactions.try_into().unwrap();

        let mut matchups = Vec::with_capacity(NUM_CARDS);
        for attacker in cards {
            let mut attacker_matchups = Vec::with_capacity(NUM_CARDS);
            for defender in cards {
                attacker_matchups.push(Matchup::new(
                    cmd.battle_system,
                    prob_cutoff,
                    attacker,
                    defender,
                ));
            }
            matchups.push(attacker_matchups.try_into().unwrap());
        }
        let matchups = matchups.try_into().unwrap();

        Self {
            metrics: Metrics::new(module_path!()),
            max_depth,
            prob_cutoff,
            player,
            battle_system: cmd.battle_system,
            cells_blocked: cmd.blocked_cells,
            interactions,
            matchups,
        }
    }

    fn get_interactions(&self, card_idx: u8, cell: u8) -> u16 {
        self.interactions[card_idx as usize][cell as usize]
    }

    fn get_matchup(&self, attacker_idx: u8, defender_idx: u8) -> Matchup {
        self.matchups[attacker_idx as usize][defender_idx as usize]
    }
}

impl VariableState {
    fn new(cmd: &core::Setup) -> Self {
        Self {
            depth: 0,
            status: Status::WaitingPlaceCard,
            turn: cmd.starting_player,
            // initialize with an invalid card idx so that errors will panic due to out of bounds
            // instead of proceeding incorrectly
            board: [u8::MAX; core::BOARD_SIZE],
            cells_blue: 0,
            cells_red: 0,
            hand_blue: Hand::new(),
            hand_red: Hand::new(),
        }
    }
}

//**************************************************************************************************
// game logic

type Board = [u8; core::BOARD_SIZE];

#[derive(Debug, Clone, Copy)]
struct Matchup {
    attacker_value: u8,
    defender_value: u8,
    attack_win_prob: f32,
}

impl Matchup {
    fn new(
        battle_system: core::BattleSystem,
        prob_cutoff: f32,
        attacker: core::Card,
        defender: core::Card,
    ) -> Self {
        let attacker_value = if let core::CardType::Assault = attacker.card_type {
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
        };

        let defender_value = match attacker.card_type {
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
        };

        let mut attack_win_prob =
            win_probabilities::lookup(battle_system, attacker_value, defender_value);

        if attack_win_prob < prob_cutoff {
            attack_win_prob = 0.0;
        }
        if attack_win_prob > (1.0 - prob_cutoff) {
            attack_win_prob = 1.0;
        }

        Matchup {
            attacker_value,
            defender_value,
            attack_win_prob,
        }
    }
}

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
    attacker_cell: u8,
    defender_cell: u8,

    attacker_idx: u8,
    defender_idx: u8,
}

impl VariableState {
    fn evaluate(&self) -> f32 {
        let blue = self.cells_blue.count_ones() as f32;
        let red = self.cells_red.count_ones() as f32;
        match self.turn {
            core::Player::Blue => blue - red,
            core::Player::Red => red - blue,
        }
    }

    fn get_place_card_actions<'c>(
        &self,
        con: &ConstantState,
        // preallocated actions vector
        actions: &'c mut Vec<Action>,
    ) -> impl Iterator<Item = Action> + 'c {
        actions.clear();

        let hand = match self.turn {
            core::Player::Blue => &self.hand_blue,
            core::Player::Red => &self.hand_red,
        };

        // unset bits are empty cells
        let empty_cells = con.cells_blocked.0 | self.cells_blue | self.cells_red;

        for cell in 0..16 {
            if (empty_cells & 1 << cell) == 0 {
                for card in 0..5 {
                    if hand.is_set(card) {
                        actions.push(Action::PlaceCard { card, cell });
                    }
                }
            }
        }

        actions.iter().copied()
    }

    fn get_pick_battle_actions(&self) -> impl Iterator<Item = Action> {
        if let Status::WaitingPickBattle { choices, .. } = &self.status {
            choices.into_iter().map(|cell| Action::PickBattle { cell })
        } else {
            unreachable!()
        }
    }

    fn get_resolutions(&self, con: &ConstantState) -> arrayvec::ArrayVec<Resolution, 2> {
        let mut resolutions = arrayvec::ArrayVec::new();
        match self.status {
            Status::WaitingResolveBattle(ref status) => {
                let matchup = con.get_matchup(status.attacker_idx, status.defender_idx);

                if matchup.attack_win_prob != 0.0 {
                    resolutions.push(Resolution {
                        winner: core::BattleWinner::Attacker,
                        probability: matchup.attack_win_prob,
                    });
                }

                if matchup.attack_win_prob != 1.0 {
                    resolutions.push(Resolution {
                        winner: core::BattleWinner::Defender,
                        probability: 1.0 - matchup.attack_win_prob,
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
            Action::PickBattle { cell } => clone.handle_pick_battle(cell),
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
            let (hand, cells, card_idx) = match self.turn {
                core::Player::Blue => (&mut self.hand_blue, &mut self.cells_blue, card),
                core::Player::Red => (&mut self.hand_red, &mut self.cells_red, card + 5),
            };

            // remove the card from the hand
            hand.unset(card);

            // mark cell
            *cells ^= 1 << cell as u16;

            // place card onto the board
            self.board[cell as usize] = card_idx;

            self.resolve_interactions(con, cell);
        }
    }

    fn handle_resolve_battle_via_command(
        &mut self,
        con: &ConstantState,
        cmd: &core::ResolveBattle,
    ) {
        fn resolve(battle_system: core::BattleSystem, value: u8, numbers: &[u8]) -> u8 {
            match battle_system {
                core::BattleSystem::Original => {
                    let min = value << 4; // range: 00, 10, 20, ..., F0
                    let max = min | 0xF; // range: 0F, 1F, 2F, ..., FF

                    let stat1 = map_number_to_range(numbers[0], min..=max);
                    let stat2 = map_number_to_range(numbers[1], ..=stat1);
                    stat1 - stat2
                }
                core::BattleSystem::Dice { .. } => {
                    // roll {value} dice and return the sum
                    let mut sum = 0;
                    for idx in 0..value {
                        sum += numbers[idx as usize];
                    }
                    sum
                }
                core::BattleSystem::Deterministic => value,
                core::BattleSystem::Test => numbers[0],
            }
        }

        if let Status::WaitingResolveBattle(ref status) = self.status {
            let matchup = con.get_matchup(status.attacker_idx, status.defender_idx);

            let attacker_value = matchup.attacker_value;
            let defender_value = matchup.defender_value;

            let attacker_roll = resolve(con.battle_system, attacker_value, &cmd.attack_roll);
            let defender_roll = resolve(con.battle_system, defender_value, &cmd.defend_roll);

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
            let attacker_cell = status.attacker_cell;
            let defender_cell = status.defender_cell;

            // flip losing card
            let (loser_cell, winning_player) = match winner {
                core::BattleWinner::Defender | core::BattleWinner::None => {
                    self.flip(attacker_cell);
                    (attacker_cell, self.turn.opposite())
                }
                core::BattleWinner::Attacker => {
                    self.flip(defender_cell);
                    (defender_cell, self.turn)
                }
            };

            // combo flip any cards the losing card points at
            let loser = self.board[loser_cell as usize];

            // set of cells pointed to by the losing card
            let maybe_interactions = con.get_interactions(loser, loser_cell);
            // set of cells belonging to the losing player
            let losing_player_cells = match winning_player {
                core::Player::Blue => self.cells_red,
                core::Player::Red => self.cells_blue,
            };
            // intersect(&) sets together to get all cells that should be combo flipped
            let interactions = losing_player_cells & maybe_interactions;

            // combo flip those cells
            for cell in 0..16 {
                if (interactions & 1 << cell) != 0 {
                    self.flip(cell);
                }
            }

            // if the attacker won
            // resolve further interactions
            if winner == core::BattleWinner::Attacker {
                self.resolve_interactions(con, attacker_cell);
            } else {
                // next turn
                if !self.check_for_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
        }
    }

    fn handle_pick_battle(&mut self, defender_cell: u8) {
        if let Status::WaitingPickBattle { attacker_cell, .. } = self.status {
            self.resolve_battle(attacker_cell, defender_cell);
        }
    }

    fn resolve_interactions(&mut self, con: &ConstantState, attacker_cell: u8) {
        let attacker = self.board[attacker_cell as usize];

        // cells pointed to by the attacker
        let maybe_interactions = con.get_interactions(attacker, attacker_cell);
        // cells belonging to the opponent
        let opponent_cells = match self.turn {
            core::Player::Blue => self.cells_red,
            core::Player::Red => self.cells_blue,
        };
        // intersect(&) together to get all opponent cards interacted with
        let interactions = opponent_cells & maybe_interactions;

        let mut defenders = core::BoardCells::NONE;
        let mut non_defenders = core::BoardCells::NONE;
        for cell in 0..16 {
            if (interactions & 1 << cell) != 0 {
                let maybe_defender = self.board[cell as usize];

                // set of cells pointed to be the (possible) defender
                let maybe_interactions = con.get_interactions(maybe_defender, cell);
                // if the attacker is part of that set, it's a defender
                if (maybe_interactions & 1 << attacker_cell) != 0 {
                    defenders.set(cell);
                } else {
                    non_defenders.set(cell);
                }
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
        self.cells_blue ^= 1 << cell as u16;
        self.cells_red ^= 1 << cell as u16;
    }

    fn resolve_battle(&mut self, attacker_cell: u8, defender_cell: u8) {
        self.status = Status::WaitingResolveBattle(WaitingResolveBattle {
            attacker_cell,
            defender_cell,

            attacker_idx: self.board[attacker_cell as usize],
            defender_idx: self.board[defender_cell as usize],
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
