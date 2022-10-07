use tetra_master_core as core;

const MAX_DEPTH: usize = 3;

type Board = [Cell; core::BOARD_SIZE];
type Hand = [Option<core::Card>; core::HAND_SIZE];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PlaceCard(core::command::PlaceCard),
    PickBattle(core::command::PickBattle),
}

#[derive(Debug, Clone, Copy)]
struct OwnedCard {
    owner: core::Player,
    card: core::Card,
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

#[derive(Debug, Clone)]
pub enum Status {
    WaitingPlace,
    WaitingBattle {
        attacker_cell: u8,
        choices: core::BoardCells,
    },
    GameOver {
        winner: Option<core::Player>,
    },
}

#[derive(Debug, Clone)]
pub struct State {
    logging_enabled: bool,
    indent: Indent,
    depth: usize,
    status: Status,
    turn: core::Player,
    board: Board,
    hand_blue: Hand,
    hand_red: Hand,
    battle_system: core::BattleSystem,
}

impl State {
    pub fn new(
        turn: core::Player,
        blocked_cells: core::BoardCells,
        hand_blue: core::Hand,
        hand_red: core::Hand,
        battle_system: core::BattleSystem,
    ) -> Self {
        fn convert_hand([a, b, c, d, e]: core::Hand) -> Hand {
            [Some(a), Some(b), Some(c), Some(d), Some(e)]
        }

        let mut board: Board = Default::default();
        for cell in blocked_cells {
            board[cell as usize] = Cell::Blocked;
        }

        let hand_blue = convert_hand(hand_blue);
        let hand_red = convert_hand(hand_red);

        Self {
            logging_enabled: false,
            indent: Indent::new(),
            depth: 0,
            status: Status::WaitingPlace,
            turn,
            board,
            hand_blue,
            hand_red,
            battle_system,
        }
    }

    fn to_move(&self) -> core::Player {
        self.turn
    }

    fn actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        match self.status {
            Status::GameOver { .. } => unreachable!(),
            Status::WaitingPlace => {
                let hand = match self.turn {
                    core::Player::P1 => &self.hand_blue,
                    core::Player::P2 => &self.hand_red,
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
                        actions.push(Action::PlaceCard(core::command::PlaceCard {
                            card: card as u8,
                            cell: cell as u8,
                        }));
                    }
                }
            }
            Status::WaitingBattle { choices, .. } => {
                for cell in choices {
                    actions.push(Action::PickBattle(core::command::PickBattle { cell }));
                }
            }
        }
        actions
    }

    pub fn apply_in_place(&mut self, action: Action) {
        match (&self.status, action) {
            (Status::WaitingPlace, Action::PlaceCard(action)) => {
                apply_place_card_action(self, action)
            }
            (Status::WaitingBattle { .. }, Action::PickBattle(action)) => {
                apply_pick_battle_action(self, action)
            }
            _ => unreachable!("apply called with invalid status/action pair"),
        }
    }

    fn apply(&self, action: Action) -> Self {
        let mut clone = self.clone();
        clone.apply_in_place(action);
        clone.indent = clone.indent.push();
        clone.depth += 1;
        clone
    }

    fn is_terminal(&self) -> bool {
        self.depth >= MAX_DEPTH || matches!(self.status, Status::GameOver { .. })
    }

    fn utility(&self, player: core::Player) -> isize {
        let mut count = 0;
        for cell in self.board {
            if let Cell::Card(card) = cell {
                if card.owner == player {
                    count += 1;
                } else {
                    count -= 1;
                }
            }
        }
        count
    }
}

pub fn minimax_search_with_logging(mut state: State) -> Action {
    state.logging_enabled = true;
    minimax_search(state)
}

pub fn minimax_search(state: State) -> Action {
    let player = state.to_move();
    let (_value, action) = max_value(player, state, None);
    action.unwrap()
}

fn max_value(
    player: core::Player,
    state: State,
    this_action: Option<Action>,
) -> (isize, Option<Action>) {
    if state.logging_enabled {
        println!("{}max_value after {this_action:?}", state.indent);
    }
    if state.is_terminal() {
        let value = state.utility(player);
        if state.logging_enabled {
            let label = match value.cmp(&0) {
                std::cmp::Ordering::Less => "LOSS",
                std::cmp::Ordering::Greater => "WIN",
                std::cmp::Ordering::Equal => "DRAW",
            };
            println!("{}value = {value} (terminal, {})", state.indent, label);
        }
        return (value, None);
    }
    let mut value = isize::MIN;
    let mut selected_action = None;
    for action in state.actions() {
        let new_state = state.apply(action);
        let min_value = if new_state.to_move() == player {
            max_value(player, new_state, Some(action)).0
        } else {
            min_value(player, new_state, Some(action)).0
        };
        if min_value > value {
            value = min_value;
            selected_action = Some(action);
        }
    }
    if state.logging_enabled {
        println!(
            "{}value = {value} | selected_action = {selected_action:?}",
            state.indent
        );
    }
    (value, selected_action)
}

fn min_value(
    player: core::Player,
    state: State,
    this_action: Option<Action>,
) -> (isize, Option<Action>) {
    if state.logging_enabled {
        println!("{}min_value after {this_action:?}", state.indent);
    }
    if state.is_terminal() {
        let value = state.utility(player);
        if state.logging_enabled {
            let label = match value.cmp(&0) {
                std::cmp::Ordering::Less => "LOSS",
                std::cmp::Ordering::Greater => "WIN",
                std::cmp::Ordering::Equal => "DRAW",
            };
            println!("{}value = {value} (terminal, {})", state.indent, label);
        }
        return (value, None);
    }
    let mut value = isize::MAX;
    let mut selected_action = None;
    for action in state.actions() {
        let new_state = state.apply(action);
        let max_value = if new_state.to_move() == player {
            min_value(player, new_state, Some(action)).0
        } else {
            max_value(player, new_state, Some(action)).0
        };
        if max_value < value {
            value = max_value;
            selected_action = Some(action);
        }
    }
    if state.logging_enabled {
        println!(
            "{}value = {value} | selected_action = {selected_action:?}",
            state.indent
        );
    }
    (value, selected_action)
}

#[derive(Debug, Clone)]
struct Indent {
    level: usize,
}

impl Indent {
    fn new() -> Self {
        Self { level: 0 }
    }
    fn push(&self) -> Self {
        Self {
            level: self.level + 1,
        }
    }
}

impl std::fmt::Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.level {
            write!(f, "|     ")?;
        }
        Ok(())
    }
}

fn apply_place_card_action(state: &mut State, cmd: core::command::PlaceCard) {
    let hand_index = cmd.card;
    let attacker_cell = cmd.cell;

    let hand = match state.turn {
        core::Player::P1 => &mut state.hand_blue,
        core::Player::P2 => &mut state.hand_red,
    };

    // ensure cell being placed is empty
    if !matches!(state.board[attacker_cell as usize], Cell::Empty) {
        panic!("Cell is not empty {attacker_cell}");
    }

    // remove the card from the hand
    let attacker = match hand[hand_index as usize].take() {
        None => {
            panic!("Card already played {hand_index}");
        }
        Some(card) => OwnedCard {
            owner: state.turn,
            card,
        },
    };

    // place card onto the board
    state.board[attacker_cell as usize] = Cell::Card(attacker);

    resolve_interactions(state, attacker_cell);
}

fn apply_pick_battle_action(state: &mut State, cmd: core::command::PickBattle) {
    let defender_cell = cmd.cell;

    let (attacker_cell, choices) = match &state.status {
        Status::WaitingBattle {
            attacker_cell,
            choices,
        } => (*attacker_cell, choices),
        _ => unreachable!(),
    };

    // ensure input cell is a valid choice
    if choices.into_iter().all(|cell| cell != defender_cell) {
        panic!("Invalid battle pick {defender_cell}");
    }

    let winner = battle(state, attacker_cell, defender_cell);

    // if the attacker won
    // resolve further interactions
    if winner == core::BattleWinner::Attacker {
        resolve_interactions(state, attacker_cell);
    } else {
        // next turn
        if !check_for_game_over(state) {
            state.turn = state.turn.opposite();
        }
    }
}

fn resolve_interactions(state: &mut State, attacker_cell: u8) {
    let attacker = match state.board[attacker_cell as usize] {
        Cell::Card(card) => card,
        _ => unreachable!("resolve_interactions can't be called with an invalid attacker_cell"),
    };

    let mut defenders = core::BoardCells::NONE;
    let mut non_defenders = core::BoardCells::NONE;
    for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
        let defender = match state.board[defender_cell as usize] {
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

    // handle multiple possible battles
    if defenders.len() > 1 {
        state.status = Status::WaitingBattle {
            attacker_cell,
            choices: defenders,
        };
        return;
    }

    // handle battles
    let winner = defenders
        .into_iter()
        .next()
        .map(|defender_cell| battle(state, attacker_cell, defender_cell));

    // if the attacker won or if there was no battle
    // handle free flips
    if winner == Some(core::BattleWinner::Attacker) || winner.is_none() {
        for cell in non_defenders {
            let defender = match &mut state.board[cell as usize] {
                Cell::Card(card) => card,
                _ => unreachable!(),
            };
            // skip card if it's already been flipped by a battle
            if defender.owner != attacker.owner {
                flip(defender);
            }
        }
    }

    // next turn
    if !check_for_game_over(state) {
        state.turn = state.turn.opposite();
    }
}

fn check_for_game_over(state: &mut State) -> bool {
    if state.hand_blue.iter().all(Option::is_none) && state.hand_red.iter().all(Option::is_none) {
        let mut p1_cards = 0;
        let mut p2_cards = 0;

        for cell in &state.board {
            if let Cell::Card(OwnedCard { owner, .. }) = cell {
                match owner {
                    core::Player::P1 => p1_cards += 1,
                    core::Player::P2 => p2_cards += 1,
                }
            }
        }

        use std::cmp::Ordering;
        let winner = match p1_cards.cmp(&p2_cards) {
            Ordering::Greater => Some(core::Player::P1),
            Ordering::Less => Some(core::Player::P2),
            Ordering::Equal => None,
        };

        state.status = Status::GameOver { winner };

        true
    } else {
        state.status = Status::WaitingPlace;

        false
    }
}

// take out card from the given cell
// panics if there is no card in the given cell
fn take_card(state: &mut State, cell: u8) -> OwnedCard {
    match std::mem::take(&mut state.board[cell as usize]) {
        Cell::Card(card) => card,
        _ => panic!("Cell didn't have a card"),
    }
}

fn battle(state: &mut State, attacker_cell: u8, defender_cell: u8) -> core::BattleWinner {
    // temporarily take out both cards from the board to allow 2 mut references
    let mut attacker = take_card(state, attacker_cell);
    let mut defender = take_card(state, defender_cell);

    let winner = calculate_battle_result(
        state,
        (attacker_cell, attacker.card),
        (defender_cell, defender.card),
    );

    let (loser_cell, loser) = match winner {
        core::BattleWinner::Defender | core::BattleWinner::None => {
            // flip attacker
            flip(&mut attacker);
            (attacker_cell, attacker)
        }
        core::BattleWinner::Attacker => {
            // flip defender
            flip(&mut defender);
            (defender_cell, defender)
        }
    };

    // combo flip any cards the losing card points at
    for &(comboed_cell, arrow) in get_possible_neighbours(loser_cell) {
        let comboed = match &mut state.board[comboed_cell as usize] {
            Cell::Card(card) => card,
            _ => continue,
        };

        if !does_interact(loser, *comboed, arrow) {
            continue;
        }

        flip(comboed);
    }

    // place both cards back into the board
    state.board[attacker_cell as usize] = Cell::Card(attacker);
    state.board[defender_cell as usize] = Cell::Card(defender);

    winner
}

fn flip(card: &mut OwnedCard) {
    let to = card.owner.opposite();
    card.owner = to;
}

fn roll(battle_system: &mut core::BattleSystem, value: u8) -> u8 {
    match battle_system {
        core::BattleSystem::Deterministic => value,
        _ => todo!(),
    }
}

fn get_attacker(
    battle_system: &mut core::BattleSystem,
    (cell, attacker): (u8, core::Card),
) -> core::Battler {
    let (digit, value) = if let core::CardType::Assault = attacker.card_type {
        // use the highest stat
        let att = attacker.attack;
        let phy = attacker.physical_defense;
        let mag = attacker.magical_defense;
        if mag > att && mag > phy {
            (core::Digit::MagicalDefense, mag)
        } else if phy > att {
            (core::Digit::PhysicalDefense, phy)
        } else {
            (core::Digit::Attack, att)
        }
    } else {
        // otherwise use the attack stat
        (core::Digit::Attack, attacker.attack)
    };

    let roll = roll(battle_system, value);
    core::Battler {
        cell,
        digit,
        value,
        roll,
    }
}

fn get_defender(
    battle_system: &mut core::BattleSystem,
    (_, attacker): (u8, core::Card),
    (cell, defender): (u8, core::Card),
) -> core::Battler {
    let (digit, value) = match attacker.card_type {
        core::CardType::Physical => (core::Digit::PhysicalDefense, defender.physical_defense),
        core::CardType::Magical => (core::Digit::MagicalDefense, defender.magical_defense),
        core::CardType::Exploit => {
            // use the lowest defense stat
            if defender.physical_defense < defender.magical_defense {
                (core::Digit::PhysicalDefense, defender.physical_defense)
            } else {
                (core::Digit::MagicalDefense, defender.magical_defense)
            }
        }
        core::CardType::Assault => {
            // use the lowest stat
            let att = defender.attack;
            let phy = defender.physical_defense;
            let mag = defender.magical_defense;
            if att < phy && att < mag {
                (core::Digit::Attack, att)
            } else if phy < mag {
                (core::Digit::PhysicalDefense, phy)
            } else {
                (core::Digit::MagicalDefense, mag)
            }
        }
    };

    let roll = roll(battle_system, value);
    core::Battler {
        cell,
        digit,
        value,
        roll,
    }
}

fn calculate_battle_result(
    state: &mut State,
    attacker_pos: (u8, core::Card),
    defender_pos: (u8, core::Card),
) -> core::BattleWinner {
    let battle_system = &mut state.battle_system;

    let attacker = get_attacker(battle_system, attacker_pos);
    let defender = get_defender(battle_system, attacker_pos, defender_pos);

    use std::cmp::Ordering;
    match attacker.roll.cmp(&defender.roll) {
        Ordering::Greater => core::BattleWinner::Attacker,
        Ordering::Less => core::BattleWinner::Defender,
        Ordering::Equal => core::BattleWinner::None,
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
    match cell {
        0x0 => &[(0x1, R), (0x4, D), (0x5, DR)],
        0x1 => &[(0x0, L), (0x2, R), (0x4, DL), (0x5, D), (0x6, DR)],
        0x2 => &[(0x1, L), (0x3, R), (0x5, DL), (0x6, D), (0x7, DR)],
        0x3 => &[(0x2, L), (0x6, DL), (0x7, D)],
        0x4 => &[(0x0, U), (0x1, UR), (0x5, R), (0x8, D), (0x9, DR)],
        0x5 => &[
            (0x0, UL),
            (0x1, U),
            (0x2, UR),
            (0x4, L),
            (0x6, R),
            (0x8, DL),
            (0x9, D),
            (0xA, DR),
        ],
        0x6 => &[
            (0x1, UL),
            (0x2, U),
            (0x3, UR),
            (0x5, L),
            (0x7, R),
            (0x9, DL),
            (0xA, D),
            (0xB, DR),
        ],
        0x7 => &[(0x3, U), (0xB, D), (0xA, DL), (0x6, L), (0x2, UL)],
        0x8 => &[(0x4, U), (0x5, UR), (0x9, R), (0xD, DR), (0xC, D)],
        0x9 => &[
            (0x5, U),
            (0x6, UR),
            (0xA, R),
            (0xE, DR),
            (0xD, D),
            (0xC, DL),
            (0x8, L),
            (0x4, UL),
        ],
        0xA => &[
            (0x6, U),
            (0x7, UR),
            (0xB, R),
            (0xF, DR),
            (0xE, D),
            (0xD, DL),
            (0x9, L),
            (0x5, UL),
        ],
        0xB => &[(0x6, UL), (0x7, U), (0xA, L), (0xE, DL), (0xF, D)],
        0xC => &[(0x8, U), (0x9, UR), (0xD, R)],
        0xD => &[(0x8, UL), (0x9, U), (0xA, UR), (0xC, L), (0xE, R)],
        0xE => &[(0x9, UL), (0xA, U), (0xB, UR), (0xD, L), (0xF, R)],
        0xF => &[(0xA, UL), (0xB, U), (0xE, L)],
        _ => unreachable!(),
    }
}
