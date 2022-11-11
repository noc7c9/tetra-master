use rand::{seq::IteratorRandom as _, Rng as _, SeedableRng as _};
use rand_pcg::Pcg32 as Rng;

use tetra_master_core as core;

use crate::interactions;
use crate::win_probabilities;

// Move number at which point to switch from MCTS to Expectiminimax
const SWITCH_POINT: u8 = 5;

// MCTS
const C: f32 = std::f32::consts::SQRT_2; // exploration factor
const MAX_TIME: u128 = 250; // time limit for MCTS (in milliseconds)

// Expectiminimax
// max tree depth (only counting place-card nodes)
const MAX_DEPTH: usize = 10 - SWITCH_POINT as usize;

type PreAlloc = Vec<Action>;
type PreAllocFull = [PreAlloc; MAX_DEPTH];

pub struct Ai {
    prealloc: PreAllocFull,
    rng: Rng,
    con: ConstantState,
    var: VariableState,
    root: Node,
    move_number: u8,
}

pub fn init(player: core::Player, setup: &core::Setup) -> Ai {
    Ai {
        prealloc: prealloc(setup),
        rng: Rng::from_entropy(),
        con: ConstantState::new(player, setup),
        var: VariableState::new(setup),
        root: Node::new(None, NodeId::Root),
        move_number: 0,
    }
}

impl Ai {
    pub fn init(player: core::Player, setup: &core::Setup) -> Self {
        init(player, setup)
    }

    pub fn reinit(&mut self, player: core::Player, setup: &core::Setup) {
        self.con = ConstantState::new(player, setup);
        self.var = VariableState::new(setup);
        self.root = Node::new(None, NodeId::Root);
        self.move_number = 0;
    }

    fn has_switched(&self) -> bool {
        self.move_number >= SWITCH_POINT
    }
}

impl super::Ai for Ai {
    fn get_action(&mut self) -> crate::Action {
        let action = if self.has_switched() {
            expectiminimax_search(&mut self.prealloc, &self.con, &self.var)
        } else {
            monte_carlo_tree_search(
                &mut self.prealloc[0],
                &mut self.rng,
                &self.con,
                &self.var,
                &mut self.root,
            )
        };

        let player = self.con.player;
        match action {
            Action::PlaceCard { cell, card } => {
                let cell = cell.0;
                let card = card.0;
                crate::Action::PlaceCard(core::PlaceCard { player, cell, card })
            }
            Action::PickBattle { cell } => {
                let cell = cell.0;
                crate::Action::PickBattle(core::PickBattle { player, cell })
            }
        }
    }

    fn apply_place_card(&mut self, cmd: core::PlaceCard) {
        self.move_number += 1;

        let cell = CellIdx::new(cmd.cell);
        let card = CardHandIdx::new(cmd.card);

        if !self.has_switched() {
            let id = NodeId::Action(Action::PlaceCard { cell, card });
            self.root = self
                .root
                .take_child_by_id(&mut self.prealloc[0], &self.con, &self.var, id);
        }

        self.var.handle_place_card(&self.con, cell, card);
    }

    fn apply_pick_battle(&mut self, cmd: core::PickBattle) {
        let cell = CellIdx::new(cmd.cell);

        if !self.has_switched() {
            let id = NodeId::Action(Action::PickBattle { cell });
            self.root = self
                .root
                .take_child_by_id(&mut self.prealloc[0], &self.con, &self.var, id);
        }

        self.var.handle_pick_battle(cell);
    }

    fn apply_resolve_battle(&mut self, cmd: &core::ResolveBattle) {
        let winner = self.var.resolve_battle_to_winner(&self.con, cmd);

        if !self.has_switched() {
            let id = NodeId::Resolution(Resolution {
                winner,
                probability: 0., // doesn't matter
            });

            self.root = self
                .root
                .take_child_by_id(&mut self.prealloc[0], &self.con, &self.var, id);
        }

        self.var.handle_resolve_battle(&self.con, winner);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    PlaceCard { cell: CellIdx, card: CardHandIdx },
    PickBattle { cell: CellIdx },
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::PlaceCard { card, cell, .. } => {
                write!(f, "Place({:X}, {:X})", card.0, cell.0)
            }
            Action::PickBattle { cell, .. } => {
                write!(f, "Pick({:X})    ", cell.0)
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
// MCTS logic

#[derive(Debug, Clone, Copy)]
enum NodeId {
    Root,
    Action(Action),
    Resolution(Resolution),
}

impl PartialEq for NodeId {
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (Self::Root, Self::Root) => true,
            (Self::Action(a), Self::Action(b)) => a == b,
            (Self::Resolution(a), Self::Resolution(b)) => {
                use tetra_master_core::BattleWinner::*;
                matches!(
                    (a.winner, b.winner),
                    // note: draws go in favor of the defender
                    (Attacker, Attacker) | (Defender | None, Defender | None)
                )
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    children: Vec<Node>,

    // who made the action that created this node
    owner: Option<core::Player>,
    id: NodeId,

    visits: u32,
    score: f32,
}

impl Node {
    fn new(owner: Option<core::Player>, id: NodeId) -> Self {
        Self {
            children: vec![],

            owner,
            id,

            visits: 0,
            score: 0.,
        }
    }

    fn take_child_by_id(
        &mut self,
        prealloc: &mut PreAlloc,
        con: &ConstantState,
        var: &VariableState,
        id: NodeId,
    ) -> Node {
        self.expand(prealloc, con, var);
        let idx = self.children.iter().position(|c| c.id == id).unwrap();
        self.children.swap_remove(idx)
    }

    fn expand(&mut self, prealloc: &mut PreAlloc, con: &ConstantState, var: &VariableState) {
        if !self.children.is_empty() {
            // already expanded
            return;
        }

        debug_assert!(self.children.capacity() == 0);

        let owner = Some(var.turn);
        match &var.status {
            Status::GameOver => unreachable!(),
            Status::WaitingPlaceCard => {
                let iter = var.get_place_card_actions(con, prealloc);
                self.children.reserve_exact(iter.len());
                for action in iter {
                    let child = Node::new(owner, NodeId::Action(action));
                    self.children.push(child);
                }
            }
            Status::WaitingPickBattle { .. } => {
                let iter = var.get_pick_battle_actions();
                self.children.reserve_exact(iter.len());
                for action in iter {
                    let child = Node::new(owner, NodeId::Action(action));
                    self.children.push(child);
                }
            }
            Status::WaitingResolveBattle(_) => {
                let iter = var.get_resolutions(con);
                self.children.reserve_exact(iter.len());
                for resolution in iter {
                    let child = Node::new(owner, NodeId::Resolution(resolution));
                    self.children.push(child);
                }
            }
        }
    }

    fn update(&mut self, winner: Option<core::Player>) {
        self.visits += 1;
        let score = match winner {
            winner if winner == self.owner => 2.,
            None => 1.,
            _ => 0.,
        };
        if let NodeId::Resolution(res) = self.id {
            self.score += res.probability * score;
        } else {
            self.score += score;
        }
    }

    fn get_child_with_most_visits(&self) -> &Node {
        self.children.iter().max_by_key(|c| c.visits).unwrap()
    }

    fn get_child_with_highest_uct(&mut self) -> &mut Node {
        let parent_visits = self.visits;
        self.children
            .iter_mut()
            .max_by(|a, b| {
                let a = a.uct_value(parent_visits);
                let b = b.uct_value(parent_visits);
                a.total_cmp(&b)
            })
            .unwrap()
    }

    fn uct_value(&self, parent_visits: u32) -> f32 {
        if self.visits == 0 {
            f32::INFINITY
        } else {
            let score = self.score as f32;
            let visits = self.visits as f32;
            let parent_visits = parent_visits as f32;
            (score / visits) + C * (parent_visits.ln() / visits).sqrt()
        }
    }
}

fn monte_carlo_tree_search(
    prealloc: &mut PreAlloc,
    rng: &mut Rng,
    con: &ConstantState,
    var: &VariableState,
    root: &mut Node,
) -> Action {
    fn simulate_random_playout(
        prealloc: &mut PreAlloc,
        rng: &mut Rng,
        con: &ConstantState,
        var: &mut VariableState,
    ) -> Option<core::Player> {
        loop {
            match &var.status {
                Status::GameOver => break,
                Status::WaitingPlaceCard => {
                    if let Action::PlaceCard { cell, card } = var
                        .get_place_card_actions(con, prealloc)
                        .choose(rng)
                        .unwrap()
                    {
                        var.handle_place_card(con, cell, card);
                    } else {
                        unreachable!()
                    }
                }
                Status::WaitingPickBattle { .. } => {
                    if let Action::PickBattle { cell } =
                        var.get_pick_battle_actions().choose(rng).unwrap()
                    {
                        var.handle_pick_battle(cell);
                    } else {
                        unreachable!()
                    }
                }
                Status::WaitingResolveBattle(_) => {
                    let resolutions = var.get_resolutions(con);
                    let resolution = if rng.gen_bool(resolutions[0].probability.into()) {
                        resolutions[0]
                    } else {
                        resolutions[1]
                    };
                    var.handle_resolve_battle(con, resolution.winner);
                }
            }
        }
        var.get_winner()
    }

    fn recur(
        prealloc: &mut PreAlloc,
        rng: &mut Rng,
        con: &ConstantState,
        var: &mut VariableState,
        node: &mut Node,
        just_expanded: bool,
    ) -> Option<core::Player> {
        // This is a leaf node
        if node.children.is_empty() {
            if !var.status.is_game_over() {
                // newly expanded node
                if just_expanded {
                    // Phase 3 - Simulation
                    let playout_result = simulate_random_playout(prealloc, rng, con, var);

                    // update this node and return to allow for back propogation
                    node.update(playout_result);
                    playout_result
                }
                // not newly expanded
                else {
                    // Phase 2 - Expansion
                    node.expand(prealloc, con, var);

                    let random_idx = rng.gen_range(0..node.children.len());
                    let random_child = &mut node.children[random_idx];
                    var.handle_node_id(con, random_child.id);

                    let playout_result = recur(prealloc, rng, con, var, random_child, true);

                    // update this node and return to allow for back propogation
                    node.update(playout_result);
                    playout_result
                }
            } else {
                // This is a terminal node
                // so skip expansion and simulation
                // and return the status directly
                let playout_result = var.get_winner();
                node.update(playout_result);
                playout_result
            }
        }
        // This is a *not* leaf node
        else {
            // Phase 1 - Selection
            let selected_node = node.get_child_with_highest_uct();
            var.handle_node_id(con, selected_node.id);

            let playout_result = recur(prealloc, rng, con, var, selected_node, false);

            // update this node and return to allow for back propogation
            node.update(playout_result);
            playout_result
        }
    }

    let now = std::time::Instant::now();
    while now.elapsed().as_millis() < MAX_TIME {
        recur(prealloc, rng, con, &mut var.clone(), root, false);
    }

    match root.get_child_with_most_visits().id {
        NodeId::Action(action) => action,
        _ => unreachable!(),
    }
}

//**************************************************************************************************
// expectiminimax logic

fn expectiminimax_search(
    prealloc: &mut [PreAlloc],
    con: &ConstantState,
    var: &VariableState,
) -> Action {
    reset!();
    indent!(module_path!());

    debug_assert!(con.player == var.turn);

    let var = var.clone();

    // same logic as max_value but also tracks which move has the highest value
    let (mut alpha, beta) = (f32::NEG_INFINITY, f32::INFINITY);
    let mut curr_value = f32::NEG_INFINITY;

    macro_rules! select_action {
        ($prealloc:expr, $apply:ident, $actions_iter:expr) => {{
            let mut selected_action = None;
            for action in $actions_iter {
                indent!("{action:?}");
                let new_state_value =
                    state_value($prealloc, con, var.apply_action(con, action), alpha, beta);
                dedent!("{action:?} | {new_state_value}");

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
            select_action!(
                rest,
                apply_place_card,
                var.get_place_card_actions(con, &mut head[0])
            )
        }
        Status::WaitingPickBattle { .. } => {
            select_action!(prealloc, apply_pick_battle, var.get_pick_battle_actions())
        }
        _ => unreachable!(),
    };

    log!("SELECTED {selected_action} | {curr_value}");
    dedent!();

    selected_action
}

#[inline(always)]
fn state_value(
    prealloc: &mut [PreAlloc],
    con: &ConstantState,
    var: VariableState,
    alpha: f32,
    beta: f32,
) -> f32 {
    match &var.status {
        Status::WaitingResolveBattle(_) => chance_value(prealloc, con, var, alpha, beta),
        Status::WaitingPlaceCard { .. } => -negamax_value(prealloc, con, var, -beta, -alpha),
        Status::WaitingPickBattle { .. } => negamax_value(prealloc, con, var, alpha, beta),
        Status::GameOver => {
            let value = var.get_score();
            log!("TERMINAL | {value}");
            value
        }
    }
}

fn negamax_value(
    prealloc: &mut [PreAlloc],
    con: &ConstantState,
    var: VariableState,
    mut alpha: f32,
    beta: f32,
) -> f32 {
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
    prealloc: &mut [PreAlloc],
    con: &ConstantState,
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
    } else {
        // if there is only one node, short-circuit
        indent!("CHANCE alpha({alpha}) beta({beta}) | short-circuit");
        let new_var = var.apply_resolution(con, resolutions[0]);
        let value = state_value(prealloc, con, new_var, alpha, beta);
        dedent!("CHANCE | {value}");
        return value;
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
    player: core::Player,
    battle_system: core::BattleSystem,
    cells_blocked: CellSet,
    // all interactions for each card in the game
    interactions: [[CellSet; core::BOARD_SIZE]; NUM_CARDS],
    // all the matchups between each pair of cards in the game
    matchups: [[Matchup; NUM_CARDS]; NUM_CARDS],
}

// Game state which is variable
#[derive(Debug, Clone)]
struct VariableState {
    status: Status,
    turn: core::Player,
    board: Board,
    cells_blue: CellSet,
    cells_red: CellSet,
    hand_blue: Hand,
    hand_red: Hand,
}

fn prealloc(cmd: &core::Setup) -> PreAllocFull {
    let hand_size = core::HAND_SIZE;
    let board_size = core::BOARD_SIZE - cmd.blocked_cells.len();
    let max_moves = hand_size * board_size;

    let mut vecs = Vec::with_capacity(MAX_DEPTH);
    for _ in 0..MAX_DEPTH {
        vecs.push(Vec::with_capacity(max_moves));
    }
    vecs.try_into().unwrap()
}

impl ConstantState {
    fn new(player: core::Player, cmd: &core::Setup) -> Self {
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

        let interactions = {
            let mut interactions = Vec::with_capacity(NUM_CARDS);
            for card in cards {
                let mut card_interactions = Vec::with_capacity(core::BOARD_SIZE);
                for cell in 0..core::BOARD_SIZE {
                    let interactions = interactions::lookup(card.arrows, cell as u8);
                    card_interactions.push(CellSet::new(interactions));
                }
                interactions.push(card_interactions.try_into().unwrap());
            }
            interactions.try_into().unwrap()
        };

        let matchups = {
            let mut matchups = Vec::with_capacity(NUM_CARDS);
            for attacker in cards {
                let mut attacker_matchups = Vec::with_capacity(NUM_CARDS);
                for defender in cards {
                    attacker_matchups.push(Matchup::new(cmd.battle_system, attacker, defender));
                }
                matchups.push(attacker_matchups.try_into().unwrap());
            }
            matchups.try_into().unwrap()
        };

        Self {
            player,
            battle_system: cmd.battle_system,
            cells_blocked: CellSet::new(cmd.blocked_cells.0),
            interactions,
            matchups,
        }
    }

    fn get_interactions(&self, card: CardLookupIdx, cell: CellIdx) -> CellSet {
        self.interactions[card.0 as usize][cell.0 as usize]
    }

    fn get_matchup(&self, attacker: CardLookupIdx, defender: CardLookupIdx) -> Matchup {
        self.matchups[attacker.0 as usize][defender.0 as usize]
    }
}

impl VariableState {
    fn new(cmd: &core::Setup) -> Self {
        Self {
            status: Status::WaitingPlaceCard,
            turn: cmd.starting_player,
            board: Board::new(),
            cells_blue: CellSet::EMPTY,
            cells_red: CellSet::EMPTY,
            hand_blue: Hand::new(),
            hand_red: Hand::new(),
        }
    }
}

//**************************************************************************************************
// game logic

// src: https://lemire.me/blog/2018/02/21/iterating-over-set-bits-quickly/
// using an Iterator failed to optimize :(
macro_rules! iter_set_bits {
    ($bits:expr, |$item:ident| $body:expr) => {{
        let mut bits = $bits;
        while bits != 0 {
            let $item = bits.trailing_zeros(); // get index of least significant set bit
            bits &= bits - 1; // clear least significant bit
            $body
        }
    }};
}

macro_rules! iter_unset_bits {
    ($bits:expr, |$item:ident| $body:expr) => {
        iter_set_bits!(!$bits, |$item| $body)
    };
}

#[derive(Debug, Clone, Copy)]
struct Matchup {
    attacker_value: u8,
    defender_value: u8,
    attack_win_prob: f32,
}

impl Matchup {
    fn new(battle_system: core::BattleSystem, attacker: core::Card, defender: core::Card) -> Self {
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

        let attack_win_prob =
            win_probabilities::lookup(battle_system, attacker_value, defender_value);

        Matchup {
            attacker_value,
            defender_value,
            attack_win_prob,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CellIdx(u8);

impl CellIdx {
    #[inline(always)]
    fn new(idx: u8) -> Self {
        debug_assert!(idx < core::BOARD_SIZE as u8);
        Self(idx)
    }
}

// Index into the interactions and matchups lookup tables
#[derive(Debug, Clone, Copy)]
struct CardLookupIdx(u8);

impl CardLookupIdx {
    const INVALID: Self = Self(u8::MAX);
}

// Index into the hand arrays (ie. the regular index used outside the AI)
#[derive(Debug, Clone, Copy, PartialEq)]
struct CardHandIdx(u8);

impl CardHandIdx {
    #[inline(always)]
    fn new(idx: u8) -> Self {
        debug_assert!(idx < core::HAND_SIZE as u8);
        Self(idx)
    }

    #[inline(always)]
    fn to_lookup_idx(self, owner: core::Player) -> CardLookupIdx {
        match owner {
            core::Player::Blue => CardLookupIdx(self.0),
            core::Player::Red => CardLookupIdx(self.0 + core::HAND_SIZE as u8),
        }
    }
}

#[derive(Debug, Clone)]
struct Board([CardLookupIdx; core::BOARD_SIZE]);

impl Board {
    fn new() -> Self {
        // initialize with an invalid card idx so that incorrect usage will panic due to out of
        // bounds instead of proceeding silently
        Self([CardLookupIdx::INVALID; core::BOARD_SIZE])
    }

    #[inline(always)]
    fn get(&self, cell: CellIdx) -> CardLookupIdx {
        self.0[cell.0 as usize]
    }

    #[inline(always)]
    fn set(&mut self, cell: CellIdx, card: CardLookupIdx) {
        self.0[cell.0 as usize] = card;
    }
}

// Bitset representing a set of board cells
#[derive(Debug, Clone, Copy)]
struct CellSet(u16);

impl CellSet {
    const EMPTY: Self = Self(0);

    #[inline(always)]
    fn new(cells: u16) -> Self {
        Self(cells)
    }

    #[inline(always)]
    fn set(&mut self, idx: CellIdx) {
        self.0 |= 1 << idx.0;
    }

    #[inline(always)]
    fn flip(&mut self, idx: CellIdx) {
        self.0 ^= 1 << idx.0;
    }

    #[inline(always)]
    fn is_set(self, idx: CellIdx) -> bool {
        (self.0 & 1 << idx.0) != 0
    }

    #[inline(always)]
    fn len(self) -> u32 {
        self.0.count_ones()
    }

    fn iter(self) -> CellSetIter {
        CellSetIter(self.0)
    }
}

impl std::ops::BitAnd for CellSet {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl std::ops::BitOr for CellSet {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::Not for CellSet {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

struct CellSetIter(u16);

impl Iterator for CellSetIter {
    type Item = CellIdx;

    fn next(&mut self) -> Option<CellIdx> {
        if self.0 == 0 {
            None
        } else {
            let item = self.0.trailing_zeros(); // get index of least significant set bit
            self.0 &= self.0 - 1; // clear least significant set bit
            Some(CellIdx(item as u8))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for CellSetIter {
    fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
}

// Bitset where set bits indicate the card has not been placed
#[derive(Debug, Clone, Copy)]
struct Hand(u8);

impl Hand {
    fn new() -> Self {
        // 5 cards in hand at start
        Self(0b0001_1111)
    }

    #[inline(always)]
    fn unset(&mut self, idx: CardHandIdx) {
        self.0 &= !(1 << idx.0);
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
        attacker_cell: CellIdx,
        choices: CellSet,
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
    attacker_cell: CellIdx,
    defender_cell: CellIdx,

    attacker_idx: CardLookupIdx,
    defender_idx: CardLookupIdx,
}

impl VariableState {
    fn get_score(&self) -> f32 {
        let blue = self.cells_blue.len() as f32;
        let red = self.cells_red.len() as f32;
        match self.turn {
            core::Player::Blue => blue - red,
            core::Player::Red => red - blue,
        }
    }

    fn get_winner(&self) -> Option<core::Player> {
        let blue = self.cells_blue.len();
        let red = self.cells_red.len();
        match blue.cmp(&red) {
            std::cmp::Ordering::Greater => Some(core::Player::Blue),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Less => Some(core::Player::Red),
        }
    }

    fn get_place_card_actions<'c>(
        &self,
        con: &ConstantState,
        actions: &'c mut PreAlloc,
    ) -> impl ExactSizeIterator<Item = Action> + 'c {
        actions.clear();

        let hand = match self.turn {
            core::Player::Blue => self.hand_blue,
            core::Player::Red => self.hand_red,
        };

        // unset bits are empty cells
        let empty_cells = con.cells_blocked | self.cells_blue | self.cells_red;

        iter_unset_bits!(empty_cells.0, |cell| {
            let cell = CellIdx::new(cell as u8);
            iter_set_bits!(hand.0, |card| {
                let card = CardHandIdx::new(card as u8);
                actions.push(Action::PlaceCard { card, cell });
            });
        });

        actions.iter().copied()
    }

    fn get_pick_battle_actions(&self) -> impl ExactSizeIterator<Item = Action> {
        if let Status::WaitingPickBattle { choices, .. } = &self.status {
            choices.iter().map(|cell| Action::PickBattle { cell })
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
        clone.handle_action(con, action);
        clone
    }

    fn apply_resolution(&self, con: &ConstantState, resolution: Resolution) -> Self {
        let mut clone = self.clone();
        clone.handle_resolve_battle(con, resolution.winner);
        clone
    }

    fn handle_node_id(&mut self, con: &ConstantState, node_id: NodeId) {
        match node_id {
            NodeId::Action(action) => self.handle_action(con, action),
            NodeId::Resolution(resolution) => self.handle_resolve_battle(con, resolution.winner),
            _ => unreachable!(),
        }
    }

    fn handle_action(&mut self, con: &ConstantState, action: Action) {
        match action {
            Action::PlaceCard { cell, card } => self.handle_place_card(con, cell, card),
            Action::PickBattle { cell } => self.handle_pick_battle(cell),
        }
    }

    fn handle_place_card(&mut self, con: &ConstantState, cell: CellIdx, card: CardHandIdx) {
        if let Status::WaitingPlaceCard = self.status {
            let card_idx = match self.turn {
                core::Player::Blue => {
                    self.hand_blue.unset(card);
                    self.cells_blue.set(cell);
                    card.to_lookup_idx(core::Player::Blue)
                }
                core::Player::Red => {
                    self.hand_red.unset(card);
                    self.cells_red.set(cell);
                    card.to_lookup_idx(core::Player::Red)
                }
            };

            // place card onto the board
            self.board.set(cell, card_idx);

            self.resolve_interactions(con, cell);
        }
    }

    fn handle_pick_battle(&mut self, defender_cell: CellIdx) {
        if let Status::WaitingPickBattle { attacker_cell, .. } = self.status {
            self.resolve_battle(attacker_cell, defender_cell);
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
            let loser = self.board.get(loser_cell);

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
            iter_set_bits!(interactions.0, |cell| {
                self.flip(CellIdx::new(cell as u8));
            });

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

    fn resolve_battle_to_winner(
        &mut self,
        con: &ConstantState,
        cmd: &core::ResolveBattle,
    ) -> core::BattleWinner {
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
            match attacker_roll.cmp(&defender_roll) {
                Ordering::Greater => core::BattleWinner::Attacker,
                Ordering::Less => core::BattleWinner::Defender,
                Ordering::Equal => core::BattleWinner::None,
            }
        } else {
            unreachable!()
        }
    }

    fn resolve_interactions(&mut self, con: &ConstantState, attacker_cell: CellIdx) {
        let attacker = self.board.get(attacker_cell);

        // cells pointed to by the attacker
        let maybe_interactions = con.get_interactions(attacker, attacker_cell);
        // cells belonging to the opponent
        let opponent_cells = match self.turn {
            core::Player::Blue => self.cells_red,
            core::Player::Red => self.cells_blue,
        };
        // intersect(&) together to get all opponent cards interacted with
        let interactions = opponent_cells & maybe_interactions;

        // Note: replacing these with CellSets results in a slow down for some reason
        let mut defenders = core::BoardCells::NONE;
        let mut non_defenders = core::BoardCells::NONE;
        iter_set_bits!(interactions.0, |cell| {
            let cell_idx = CellIdx::new(cell as u8);
            let maybe_defender = self.board.get(cell_idx);

            // set of cells pointed to be the (possible) defender
            let maybe_interactions = con.get_interactions(maybe_defender, cell_idx);
            // if the attacker is part of that set, it's a defender
            if maybe_interactions.is_set(attacker_cell) {
                defenders.set(cell as u8);
            } else {
                non_defenders.set(cell as u8);
            }
        });

        match defenders.0.count_ones() {
            0 => {
                // no battles, flip non-defenders
                iter_set_bits!(non_defenders.0, |cell| {
                    self.flip(CellIdx::new(cell as u8));
                });

                // no more interactions found, next turn
                if !self.check_for_game_over() {
                    self.turn = self.turn.opposite();
                }
            }
            1 => {
                // handle battle
                // get index of least significant set bit
                let defender_cell = CellIdx::new(defenders.0.trailing_zeros() as u8);
                self.resolve_battle(attacker_cell, defender_cell);
            }
            _ => {
                // handle multiple possible battles
                self.status = Status::WaitingPickBattle {
                    attacker_cell,
                    choices: CellSet(defenders.0),
                };
            }
        }
    }

    fn flip(&mut self, cell: CellIdx) {
        self.cells_blue.flip(cell);
        self.cells_red.flip(cell);
    }

    fn resolve_battle(&mut self, attacker_cell: CellIdx, defender_cell: CellIdx) {
        self.status = Status::WaitingResolveBattle(WaitingResolveBattle {
            attacker_cell,
            defender_cell,

            attacker_idx: self.board.get(attacker_cell),
            defender_idx: self.board.get(defender_cell),
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
