use rand::{seq::IteratorRandom as _, Rng as _, SeedableRng as _};
use rand_pcg::Pcg32 as Rng;

use tetra_master_core as core;

use crate::interactions;
use crate::win_probabilities;

pub struct Ai {
    prealloc: Vec<Action>,
    rng: Rng,
    con: ConstantState,
    root: Node,
}

pub fn init(
    max_time_ms: u128,
    c_value: f32,
    prob_cutoff: f32,
    player: core::Player,
    setup: &core::Setup,
) -> Ai {
    Ai {
        prealloc: prealloc(setup),
        rng: Rng::from_entropy(),
        con: ConstantState::new(max_time_ms, c_value, prob_cutoff, player, setup),
        root: Node::new(None, NodeId::Root, VariableState::new(setup)),
    }
}

impl Ai {
    pub fn reinit(&mut self, player: core::Player, setup: &core::Setup) {
        let max_time_ms = self.con.max_time_ms;
        let c_value = self.con.c_value;
        let prob_cutoff = self.con.prob_cutoff;
        self.con = ConstantState::new(max_time_ms, c_value, prob_cutoff, player, setup);
        self.root = Node::new(None, NodeId::Root, VariableState::new(setup));
    }
}

impl super::Ai for Ai {
    fn get_action(&mut self) -> crate::Action {
        let player = self.con.player;
        match monte_carlo_tree_search(&mut self.prealloc, &mut self.rng, &self.con, &mut self.root)
        {
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
        let cell = CellIdx::new(cmd.cell);
        let card = CardHandIdx::new(cmd.card);
        let id = NodeId::Action(Action::PlaceCard { cell, card });

        self.root = self
            .root
            .take_child_by_id(&mut self.prealloc, &self.con, id);
    }

    fn apply_pick_battle(&mut self, cmd: core::PickBattle) {
        let cell = CellIdx::new(cmd.cell);
        let id = NodeId::Action(Action::PickBattle { cell });

        self.root = self
            .root
            .take_child_by_id(&mut self.prealloc, &self.con, id);
    }

    fn apply_resolve_battle(&mut self, cmd: &core::ResolveBattle) {
        let winner = self
            .root
            .var
            .resolve_battle_command_to_winner(&self.con, cmd);
        let id = NodeId::Resolution(Resolution {
            winner,
            probability: 0., // doesn't matter
        });

        self.root = self
            .root
            .take_child_by_id(&mut self.prealloc, &self.con, id);
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

#[derive(Debug, Clone)]
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

    visits: usize,
    score: f32,

    var: VariableState,
}

impl Node {
    fn new(owner: Option<core::Player>, id: NodeId, var: VariableState) -> Self {
        Self {
            children: vec![],

            owner,
            id,

            visits: 0,
            score: 0.,

            var,
        }
    }

    fn take_child_by_id(
        &mut self,
        prealloc: &mut Vec<Action>,
        con: &ConstantState,
        id: NodeId,
    ) -> Node {
        self.expand(prealloc, con);
        let idx = self.children.iter().position(|c| c.id == id).unwrap();
        self.children.swap_remove(idx)
    }

    fn expand(&mut self, prealloc: &mut Vec<Action>, con: &ConstantState) {
        if !self.children.is_empty() {
            // already expanded
            return;
        }

        let owner = Some(self.var.turn);
        match &self.var.status {
            Status::GameOver => unreachable!(),
            Status::WaitingPlaceCard => {
                for action in self.var.get_place_card_actions(con, prealloc) {
                    let new_var = self.var.apply_action(con, action);
                    let child = Node::new(owner, NodeId::Action(action), new_var);
                    self.children.push(child);
                }
            }
            Status::WaitingPickBattle { .. } => {
                for action in self.var.get_pick_battle_actions() {
                    let new_var = self.var.apply_action(con, action);
                    let child = Node::new(owner, NodeId::Action(action), new_var);
                    self.children.push(child);
                }
            }
            Status::WaitingResolveBattle(_) => {
                for resolution in self.var.get_resolutions(con) {
                    let new_var = self.var.apply_resolution(con, resolution);
                    let child = Node::new(owner, NodeId::Resolution(resolution), new_var);
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

    fn get_child_with_highest_uct(&mut self, c_value: f32) -> &mut Node {
        let parent_visits = self.visits;
        self.children
            .iter_mut()
            .max_by(|a, b| {
                let a = a.uct_value(c_value, parent_visits);
                let b = b.uct_value(c_value, parent_visits);
                a.total_cmp(&b)
            })
            .unwrap()
    }

    fn uct_value(&self, c_value: f32, parent_visits: usize) -> f32 {
        if self.visits == 0 {
            f32::INFINITY
        } else {
            let score = self.score as f32;
            let visits = self.visits as f32;
            let parent_visits = parent_visits as f32;
            (score / visits) + c_value * (parent_visits.ln() / visits).sqrt()
        }
    }
}

fn monte_carlo_tree_search(
    prealloc: &mut Vec<Action>,
    rng: &mut Rng,
    con: &ConstantState,
    root: &mut Node,
) -> Action {
    fn simulate_random_playout(
        prealloc: &mut Vec<Action>,
        rng: &mut Rng,
        con: &ConstantState,
        var: &VariableState,
    ) -> Option<core::Player> {
        let mut var = var.clone();

        loop {
            match &var.status {
                Status::GameOver => break,
                Status::WaitingPlaceCard => {
                    if let Some(Action::PlaceCard { cell, card }) =
                        var.get_place_card_actions(con, prealloc).choose(rng)
                    {
                        var.handle_place_card(con, cell, card);
                    }
                }
                Status::WaitingPickBattle { .. } => {
                    if let Some(Action::PickBattle { cell }) =
                        var.get_pick_battle_actions().choose(rng)
                    {
                        var.handle_pick_battle(cell);
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
        prealloc: &mut Vec<Action>,
        rng: &mut Rng,
        con: &ConstantState,
        node: &mut Node,
        just_expanded: bool,
    ) -> Option<core::Player> {
        // This is a leaf node
        if node.children.is_empty() {
            if !node.var.status.is_game_over() {
                // newly expanded node
                if just_expanded {
                    // Phase 3 - Simulation
                    let playout_result = simulate_random_playout(prealloc, rng, con, &node.var);

                    // update this node and return to allow for back propogation
                    node.update(playout_result);
                    playout_result
                }
                // not newly expanded
                else {
                    // Phase 2 - Expansion
                    node.expand(prealloc, con);

                    let random_idx = rng.gen_range(0..node.children.len());
                    let random_child = &mut node.children[random_idx];
                    let playout_result = recur(prealloc, rng, con, random_child, true);

                    // update this node and return to allow for back propogation
                    node.update(playout_result);
                    playout_result
                }
            } else {
                // This is a terminal node
                // so skip expansion and simulation
                // and return the status directly
                let playout_result = node.var.get_winner();
                node.update(playout_result);
                playout_result
            }
        }
        // This is a *not* leaf node
        else {
            // Phase 1 - Selection
            let selected_node = node.get_child_with_highest_uct(con.c_value);

            let playout_result = recur(prealloc, rng, con, selected_node, false);

            // update this node and return to allow for back propogation
            node.update(playout_result);
            playout_result
        }
    }

    let now = std::time::Instant::now();
    while now.elapsed().as_millis() < con.max_time_ms {
        recur(prealloc, rng, con, root, false);
    }

    match root.get_child_with_most_visits().id {
        NodeId::Action(action) => action,
        _ => unreachable!(),
    }
}

//**************************************************************************************************
// state

const NUM_CARDS: usize = core::HAND_SIZE * 2;

// Game state which remains constant
#[derive(Debug)]
struct ConstantState {
    max_time_ms: u128,
    c_value: f32,
    prob_cutoff: f32,
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
    depth: u8,
    status: Status,
    turn: core::Player,
    board: Board,
    cells_blue: CellSet,
    cells_red: CellSet,
    hand_blue: Hand,
    hand_red: Hand,
}

fn prealloc(cmd: &core::Setup) -> Vec<Action> {
    let hand_size = core::HAND_SIZE;
    let board_size = core::BOARD_SIZE - cmd.blocked_cells.len();
    let max_moves = hand_size * board_size;

    Vec::with_capacity(max_moves)
}

impl ConstantState {
    fn new(
        max_time_ms: u128,
        c_value: f32,
        prob_cutoff: f32,
        player: core::Player,
        cmd: &core::Setup,
    ) -> Self {
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
                    attacker_matchups.push(Matchup::new(
                        cmd.battle_system,
                        prob_cutoff,
                        attacker,
                        defender,
                    ));
                }
                matchups.push(attacker_matchups.try_into().unwrap());
            }
            matchups.try_into().unwrap()
        };

        Self {
            max_time_ms,
            c_value,
            prob_cutoff,
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
            depth: 0,
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
        // preallocated actions vector
        actions: &'c mut Vec<Action>,
    ) -> impl Iterator<Item = Action> + 'c {
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

    fn get_pick_battle_actions(&self) -> impl Iterator<Item = Action> {
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

    fn resolve_battle_command_to_winner(
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

    fn handle_pick_battle(&mut self, defender_cell: CellIdx) {
        if let Status::WaitingPickBattle { attacker_cell, .. } = self.status {
            self.resolve_battle(attacker_cell, defender_cell);
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
