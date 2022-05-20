use crate::{
    Arrows, BattleResult, BattleStat, BattleWinner, Card, CardType, Cell, Entry, GameLog,
    GameState, GameStatus, Input, InputBattle, InputPlace, OwnedCard, Player,
};

pub(crate) fn next(state: &mut GameState, log: &mut GameLog, input: Input) -> Result<(), String> {
    match (&state.status, input) {
        (GameStatus::WaitingPlace, Input::Place(input)) => handle_waiting_place(state, log, input),
        (GameStatus::WaitingBattle { .. }, Input::Battle(input)) => {
            handle_waiting_battle(state, log, input)
        }
        _ => unreachable!("next called with invalid status/input pair"),
    }
}

fn handle_waiting_place(
    state: &mut GameState,
    log: &mut GameLog,
    input: InputPlace,
) -> Result<(), String> {
    let hand_index = input.card;
    let attacker_cell = input.cell;

    let hand = match state.turn {
        Player::P1 => &mut state.p1_hand,
        Player::P2 => &mut state.p2_hand,
    };

    // ensure cell being placed is empty
    if !matches!(state.board[attacker_cell], Cell::Empty) {
        return Err(format!("Cell {:X} is not empty", attacker_cell));
    }

    // remove the card from the hand
    let attacker = match hand[hand_index].take() {
        None => {
            return Err(format!("Card {} has already been played", hand_index));
        }
        Some(card) => OwnedCard {
            owner: state.turn,
            card,
        },
    };

    // place card onto the board
    log.append(Entry::place_card(attacker, attacker_cell));
    state.board[attacker_cell] = Cell::Card(attacker);

    resolve_rest_of_turn(state, log, attacker_cell);

    Ok(())
}

fn handle_waiting_battle(
    state: &mut GameState,
    log: &mut GameLog,
    input: InputBattle,
) -> Result<(), String> {
    let defender_cell = input.cell;

    let (attacker_cell, choices) = match &state.status {
        GameStatus::WaitingBattle {
            attacker_cell,
            choices,
        } => (*attacker_cell, choices),
        _ => unreachable!(),
    };

    // ensure input cell is a valid choice
    if choices.iter().all(|&(cell, _)| cell != defender_cell) {
        return Err(format!("Cell {:X} is not a valid choice", attacker_cell));
    }

    let winner = battle(state, log, attacker_cell, defender_cell);

    // if the attacker won
    // resolve further interactions
    if winner == BattleWinner::Attacker {
        resolve_rest_of_turn(state, log, attacker_cell);
    } else {
        state.status = GameStatus::WaitingPlace;
    }

    Ok(())
}

// common logic for both handle_waiting_place and handle_waiting_battle
fn resolve_rest_of_turn(state: &mut GameState, log: &mut GameLog, attacker_cell: usize) {
    let attacker = match state.board[attacker_cell] {
        Cell::Card(card) => card,
        _ => unreachable!("resolve_rest_of_turn can't be called with an invalid attacker_cell"),
    };

    let mut defenders = vec![];
    let mut non_defenders = vec![];
    for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
        let defender = match state.board[defender_cell] {
            Cell::Card(card) => card,
            _ => continue,
        };

        if !does_interact(attacker, defender, arrow) {
            continue;
        }

        if defender.card.arrows.has(arrow.reverse()) {
            defenders.push((defender_cell, defender.card));
        } else {
            non_defenders.push(defender_cell);
        }
    }

    // handle multiple possible battles
    if defenders.len() > 1 {
        state.status = GameStatus::WaitingBattle {
            attacker_cell,
            choices: defenders,
        };
        return;
    }

    // handle battles
    let winner = defenders
        .first()
        .map(|(defender_cell, _)| battle(state, log, attacker_cell, *defender_cell));

    // if the attacker won or if there was no battle
    // handle free flips
    if winner == Some(BattleWinner::Attacker) || winner == None {
        for cell in non_defenders {
            let defender = match &mut state.board[cell] {
                Cell::Card(card) => card,
                _ => unreachable!(),
            };
            flip(log, defender, cell, false);
        }
    }

    // next turn
    state.turn = state.turn.opposite();
    log.append(Entry::next_turn(state.turn));

    // check if the game is over
    if state.p1_hand.iter().all(Option::is_none) && state.p2_hand.iter().all(Option::is_none) {
        let mut p1_cards = 0;
        let mut p2_cards = 0;

        for cell in &state.board {
            if let Cell::Card(OwnedCard { owner, .. }) = cell {
                match owner {
                    Player::P1 => p1_cards += 1,
                    Player::P2 => p2_cards += 1,
                }
            }
        }

        use std::cmp::Ordering;
        let winner = match p1_cards.cmp(&p2_cards) {
            Ordering::Greater => Some(Player::P1),
            Ordering::Less => Some(Player::P2),
            Ordering::Equal => None,
        };

        state.status = GameStatus::GameOver { winner };
    } else {
        state.status = GameStatus::WaitingPlace;
    }
}

fn battle(
    state: &mut GameState,
    log: &mut GameLog,
    attacker_cell: usize,
    defender_cell: usize,
) -> BattleWinner {
    // temporarily take out both cards from the board to allow 2 mut references
    let mut attacker = state.take_card(attacker_cell);
    let mut defender = state.take_card(defender_cell);

    let result = calculate_battle_result(&state.rng, attacker.card, defender.card);
    log.append(Entry::battle(attacker, defender, result));
    match result.winner {
        BattleWinner::Defender | BattleWinner::None => {
            // flip attacker
            flip(log, &mut attacker, attacker_cell, false);
        }
        BattleWinner::Attacker => {
            // flip defender
            flip(log, &mut defender, defender_cell, false);

            // combo flip any cards defender points at
            for &(comboed_cell, arrow) in get_possible_neighbours(defender_cell) {
                let comboed = match &mut state.board[comboed_cell] {
                    Cell::Card(card) => card,
                    _ => continue,
                };

                if !does_interact(defender, *comboed, arrow) {
                    continue;
                }

                flip(log, comboed, comboed_cell, true);
            }
        }
    }

    // place both cards back into the board
    state.board[attacker_cell] = Cell::Card(attacker);
    state.board[defender_cell] = Cell::Card(defender);

    result.winner
}

fn flip(log: &mut GameLog, card: &mut OwnedCard, cell: usize, via_combo: bool) {
    let to = card.owner.opposite();
    log.append(Entry::flip_card(*card, cell, to, via_combo));
    card.owner = to;
}

fn get_attack_stat(rng: &fastrand::Rng, attacker: Card) -> BattleStat {
    let (digit, value) = if let CardType::Assault = attacker.card_type {
        // use the highest stat
        let att = attacker.attack;
        let phy = attacker.physical_defense;
        let mag = attacker.magical_defense;
        if mag > att && mag > phy {
            (3, mag)
        } else if phy > att {
            (2, phy)
        } else {
            (0, att)
        }
    } else {
        // otherwise use the attack stat
        (0, attacker.attack)
    };

    let roll = rng.u8(..=value);
    BattleStat { digit, value, roll }
}

fn get_defense_stat(rng: &fastrand::Rng, attacker: Card, defender: Card) -> BattleStat {
    let (digit, value) = match attacker.card_type {
        CardType::Physical => (2, defender.physical_defense),
        CardType::Magical => (3, defender.magical_defense),
        CardType::Exploit =>
        // use the lowest defense stat
        {
            if defender.physical_defense < defender.magical_defense {
                (2, defender.physical_defense)
            } else {
                (3, defender.magical_defense)
            }
        }
        CardType::Assault => {
            // use the lowest stat
            let att = defender.attack;
            let phy = defender.physical_defense;
            let mag = defender.magical_defense;
            if att < phy && att < mag {
                (0, att)
            } else if phy < mag {
                (2, phy)
            } else {
                (3, mag)
            }
        }
    };

    let roll = rng.u8(..=value);
    BattleStat { digit, value, roll }
}

fn calculate_battle_result(rng: &fastrand::Rng, attacker: Card, defender: Card) -> BattleResult {
    let attack_stat = get_attack_stat(rng, attacker);
    let defense_stat = get_defense_stat(rng, attacker, defender);

    let att = attack_stat.resolve();
    let def = defense_stat.resolve();

    use std::cmp::Ordering;
    let winner = match att.cmp(&def) {
        Ordering::Greater => BattleWinner::Attacker,
        Ordering::Less => BattleWinner::Defender,
        Ordering::Equal => BattleWinner::None,
    };

    BattleResult {
        winner,
        attack_stat,
        defense_stat,
    }
}

fn does_interact(attacker: OwnedCard, defender: OwnedCard, arrow_to_defender: Arrows) -> bool {
    // they don't interact if both cards belong to the same player
    if defender.owner == attacker.owner {
        return false;
    }

    // they interact if the attacking card has an arrow in the direction of the defender
    attacker.card.arrows.has(arrow_to_defender)
}

// returns neighbouring cells along with the arrow that points at them
fn get_possible_neighbours(cell: usize) -> &'static [(usize, Arrows)] {
    const U: Arrows = Arrows::UP;
    const UR: Arrows = Arrows::UP_RIGHT;
    const R: Arrows = Arrows::RIGHT;
    const DR: Arrows = Arrows::DOWN_RIGHT;
    const D: Arrows = Arrows::DOWN;
    const DL: Arrows = Arrows::DOWN_LEFT;
    const L: Arrows = Arrows::LEFT;
    const UL: Arrows = Arrows::UP_LEFT;
    match cell {
        0x0 => &[(0x1, R), (0x5, DR), (0x4, D)],
        0x1 => &[(0x2, R), (0x6, DR), (0x5, D), (0x4, DL), (0x0, L)],
        0x2 => &[(0x3, R), (0x7, DR), (0x6, D), (0x5, DL), (0x1, L)],
        0x3 => &[(0x7, D), (0x6, DL), (0x2, L)],
        0x4 => &[(0x0, U), (0x1, UR), (0x5, R), (0x9, DR), (0x8, D)],
        0x5 => &[
            (0x1, U),
            (0x2, UR),
            (0x6, R),
            (0xA, DR),
            (0x9, D),
            (0x8, DL),
            (0x4, L),
            (0x0, UL),
        ],
        0x6 => &[
            (0x2, U),
            (0x3, UR),
            (0x7, R),
            (0xB, DR),
            (0xA, D),
            (0x9, DL),
            (0x5, L),
            (0x1, UL),
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
        0xB => &[(0x7, U), (0xF, D), (0xE, DL), (0xA, L), (0x6, UL)],
        0xC => &[(0x8, U), (0x9, UR), (0xD, R)],
        0xD => &[(0x9, U), (0xA, UR), (0xE, R), (0xC, L), (0x8, UL)],
        0xE => &[(0xA, U), (0xB, UR), (0xF, R), (0xD, L), (0x9, UL)],
        0xF => &[(0xB, U), (0xE, L), (0xA, UL)],
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn rng() -> fastrand::Rng {
        fastrand::Rng::new()
    }

    fn with_seed(seed: u64) -> fastrand::Rng {
        fastrand::Rng::with_seed(seed)
    }

    impl GameState {
        fn empty() -> Self {
            GameState {
                status: GameStatus::WaitingPlace,
                rng: fastrand::Rng::with_seed(0),
                turn: Player::P1,
                board: [
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Empty,
                ],
                p1_hand: [None, None, None, None, None],
                p2_hand: [None, None, None, None, None],
            }
        }
    }

    impl Card {
        fn from_str(stats: &str, arrows: Arrows) -> Self {
            let attack = 0xF + 0x10 * u8::from_str_radix(&stats[0..1], 16).unwrap();
            let card_type = match &stats[1..2] {
                "P" => CardType::Physical,
                "M" => CardType::Magical,
                "X" => CardType::Exploit,
                "A" => CardType::Assault,
                _ => unreachable!(),
            };
            let physical_defense = 0xF + 0x10 * u8::from_str_radix(&stats[2..3], 16).unwrap();
            let magical_defense = 0xF + 0x10 * u8::from_str_radix(&stats[3..4], 16).unwrap();
            Card {
                card_type,
                attack,
                physical_defense,
                magical_defense,
                arrows,
            }
        }

        fn basic() -> Self {
            Card {
                card_type: CardType::Physical,
                arrows: Arrows::NONE,
                attack: 0,
                physical_defense: 0,
                magical_defense: 0,
            }
        }
    }

    impl OwnedCard {
        fn p1(card: Card) -> Self {
            let owner = Player::P1;
            OwnedCard { owner, card }
        }

        fn p2(card: Card) -> Self {
            let owner = Player::P2;
            OwnedCard { owner, card }
        }
    }

    impl Cell {
        fn p1_card(card: Card) -> Self {
            Cell::Card(OwnedCard::p1(card))
        }

        fn p2_card(card: Card) -> Self {
            Cell::Card(OwnedCard::p2(card))
        }
    }

    impl Input {
        fn place(card: usize, cell: usize) -> Self {
            Input::Place(InputPlace { card, cell })
        }

        fn battle(cell: usize) -> Self {
            Input::Battle(InputBattle { cell })
        }
    }

    #[test]
    fn turn_should_change_after_a_valid_play() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        next(&mut state, &mut log, Input::place(0, 0)).unwrap();

        assert_eq!(state.turn, Player::P2);
    }

    #[test]
    fn reject_input_if_the_card_has_already_been_played() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [None, None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        let res = next(&mut state, &mut log, Input::place(2, 0));

        assert_eq!(res, Err("Card 2 has already been played".into()));
    }

    #[test]
    fn reject_input_if_the_cell_played_on_is_blocked() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        state.board[0xB] = Cell::Blocked;

        let res = next(&mut state, &mut log, Input::place(0, 0xB));

        assert_eq!(res, Err("Cell B is not empty".into()));
    }

    #[test]
    fn reject_input_if_the_cell_played_on_already_has_a_card_placed() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        state.board[3] = Cell::p1_card(Card::basic());

        let res = next(&mut state, &mut log, Input::place(0, 3));

        assert_eq!(res, Err("Cell 3 is not empty".into()));
    }

    #[test]
    fn move_card_from_hand_to_board_if_input_is_valid() {
        let card = Card::basic();
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        next(&mut state, &mut log, Input::place(0, 7)).unwrap();

        assert_eq!(state.p1_hand[0], None);
        assert_eq!(state.board[0x7], Cell::p1_card(card));
    }

    #[test]
    fn update_game_log_on_placing_card() {
        let card = Card::basic();
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        next(&mut state, &mut log, Input::place(0, 7)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card), 7),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn flip_cards_that_belong_to_opponent_are_pointed_to_and_dont_point_back() {
        let card_no_arrows = Card {
            arrows: Arrows::NONE,
            ..Card::basic()
        };
        let card_points_up_and_right = Card {
            arrows: Arrows::UP | Arrows::RIGHT,
            ..Card::basic()
        };
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_up_and_right), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        // should flip, is pointed to, belongs to opponent
        state.board[0] = Cell::p2_card(card_no_arrows);
        // shouldn't flip, doesn't belongs to opponent
        state.board[5] = Cell::p1_card(card_no_arrows);
        // shouldn't flip, isn't pointed to
        state.board[8] = Cell::p2_card(card_no_arrows);

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        assert_eq!(state.board[0], Cell::p1_card(card_no_arrows));
        assert_eq!(state.board[5], Cell::p1_card(card_no_arrows));
        assert_eq!(state.board[8], Cell::p2_card(card_no_arrows));
    }

    #[test]
    fn update_game_log_on_flipping_cards() {
        let card_no_arrows = Card {
            arrows: Arrows::NONE,
            ..Card::basic()
        };
        let card_points_up = Card {
            arrows: Arrows::UP | Arrows::RIGHT,
            ..Card::basic()
        };
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_up), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        state.board[0] = Cell::p2_card(card_no_arrows);

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_up), 4,),
                &Entry::flip_card(OwnedCard::p2(card_no_arrows), 0, Player::P1, false),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn battle_cards_that_belong_to_opponent_are_pointed_to_and_point_back() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("1P00", Arrows::UP);
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_up), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);

        {
            // rng is set to make the attacker win
            let mut state = GameState {
                rng: with_seed(0),
                ..state.clone()
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            assert_eq!(state.board[0], Cell::p1_card(card_points_down));
            assert_eq!(state.board[4], Cell::p1_card(card_points_up));
        }

        {
            // rng is set to make the defender win
            let mut state = GameState {
                rng: with_seed(1),
                ..state.clone()
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            assert_eq!(state.board[0], Cell::p2_card(card_points_down));
            assert_eq!(state.board[4], Cell::p2_card(card_points_up));
        }

        {
            // rng is set to make the battle draw and default as a defender win
            let mut state = GameState {
                rng: with_seed(94),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            assert_eq!(state.board[0], Cell::p2_card(card_points_down));
            assert_eq!(state.board[4], Cell::p2_card(card_points_up));
        }
    }

    #[test]
    fn update_game_log_on_battles() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("1P00", Arrows::UP);
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_up), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);

        {
            // rng is set to make the attacker win
            let mut state = GameState {
                rng: with_seed(0),
                ..state.clone()
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            let log: Vec<_> = log.iter().collect();
            assert_eq!(
                log,
                vec![
                    &Entry::next_turn(Player::P1),
                    &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                    &Entry::battle(
                        OwnedCard::p1(card_points_up),
                        OwnedCard::p2(card_points_down),
                        BattleResult {
                            winner: BattleWinner::Attacker,
                            attack_stat: BattleStat {
                                digit: 0,
                                value: 0x1F,
                                roll: 17
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 0x1F,
                                roll: 31
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p2(card_points_down), 0, Player::P1, false),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }

        {
            // rng is set to make the defender win
            let mut state = GameState {
                rng: with_seed(1),
                ..state.clone()
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            let log: Vec<_> = log.iter().collect();
            assert_eq!(
                log,
                vec![
                    &Entry::next_turn(Player::P1),
                    &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                    &Entry::battle(
                        OwnedCard::p1(card_points_up),
                        OwnedCard::p2(card_points_down),
                        BattleResult {
                            winner: BattleWinner::Defender,
                            attack_stat: BattleStat {
                                digit: 0,
                                value: 0x1F,
                                roll: 28
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 0x1F,
                                roll: 3
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p1(card_points_up), 4, Player::P2, false),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }

        {
            // rng is set to make the battle draw and default as a defender win
            let mut state = GameState {
                rng: with_seed(94),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Input::place(0, 4)).unwrap();

            let log: Vec<_> = log.iter().collect();
            assert_eq!(
                log,
                vec![
                    &Entry::next_turn(Player::P1),
                    &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                    &Entry::battle(
                        OwnedCard::p1(card_points_up),
                        OwnedCard::p2(card_points_down,),
                        BattleResult {
                            winner: BattleWinner::None,
                            attack_stat: BattleStat {
                                digit: 0,
                                value: 0x1F,
                                roll: 23
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 0x1F,
                                roll: 23
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p1(card_points_up), 4, Player::P2, false),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }
    }

    #[test]
    fn flip_other_undefended_cards_after_attacker_wins_battle() {
        let card_points_none = Card::from_str("0P00", Arrows::NONE);
        let card_points_down = Card::from_str("0P00", Arrows::DOWN);
        let card_points_all = Card::from_str("FP00", Arrows::ALL);
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_all), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[1] = Cell::p2_card(card_points_none);
        state.board[5] = Cell::p2_card(card_points_none);
        state.board[9] = Cell::p2_card(card_points_none);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        assert_eq!(state.board[0], Cell::p1_card(card_points_down));
        assert_eq!(state.board[1], Cell::p1_card(card_points_none));
        assert_eq!(state.board[5], Cell::p1_card(card_points_none));
        assert_eq!(state.board[9], Cell::p1_card(card_points_none));
        assert_eq!(state.board[4], Cell::p1_card(card_points_all));

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_all), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_all),
                    OwnedCard::p2(card_points_down,),
                    BattleResult {
                        winner: BattleWinner::Attacker,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0xFF,
                            roll: 142
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x0F,
                            roll: 15
                        },
                    }
                ),
                &Entry::flip_card(OwnedCard::p2(card_points_down), 0, Player::P1, false),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 1, Player::P1, false),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 5, Player::P1, false),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 9, Player::P1, false),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn dont_flip_other_undefended_cards_after_attacker_loses_battle() {
        let card_points_none = Card::from_str("0P00", Arrows::NONE);
        let card_points_down = Card::from_str("0PF0", Arrows::DOWN);
        let card_points_all = Card::from_str("0P00", Arrows::ALL);
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_all), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[1] = Cell::p2_card(card_points_none);
        state.board[5] = Cell::p2_card(card_points_none);
        state.board[9] = Cell::p2_card(card_points_none);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        assert_eq!(state.board[0], Cell::p2_card(card_points_down));
        assert_eq!(state.board[1], Cell::p2_card(card_points_none));
        assert_eq!(state.board[5], Cell::p2_card(card_points_none));
        assert_eq!(state.board[9], Cell::p2_card(card_points_none));
        assert_eq!(state.board[4], Cell::p2_card(card_points_all));

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_all), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_all),
                    OwnedCard::p2(card_points_down,),
                    BattleResult {
                        winner: BattleWinner::Defender,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x0F,
                            roll: 8
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0xFF,
                            roll: 109
                        },
                    }
                ),
                &Entry::flip_card(OwnedCard::p1(card_points_all), 4, Player::P2, false),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn change_status_to_chose_battle_when_multiple_battles_are_available() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("0P10", Arrows::UP);
        let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::UP | Arrows::DOWN);

        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_vert), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        assert_eq!(
            state.status,
            GameStatus::WaitingBattle {
                attacker_cell: 4,
                choices: vec![(0, card_points_down), (8, card_points_up)]
            }
        );
    }

    #[test]
    fn update_game_log_with_place_entry_when_multiple_battles_are_available() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("0P10", Arrows::UP);
        let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::UP | Arrows::DOWN);

        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_vert), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_vert), 4),
            ]
        );
    }

    #[test]
    fn continue_after_battle_choice_is_given() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("0P10", Arrows::UP);
        let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::UP | Arrows::DOWN);

        let mut state = GameState {
            // rng is set to make the attacker win both battles
            rng: with_seed(0),
            turn: Player::P1,
            p1_hand: [Some(card_points_vert), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();
        next(&mut state, &mut log, Input::battle(8)).unwrap();

        assert_eq!(state.board[0], Cell::p1_card(card_points_down));
        assert_eq!(state.board[4], Cell::p1_card(card_points_vert));
        assert_eq!(state.board[8], Cell::p1_card(card_points_up));

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_vert), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_vert),
                    OwnedCard::p2(card_points_up),
                    BattleResult {
                        winner: BattleWinner::Attacker,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x1F,
                            roll: 17
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x1F,
                            roll: 31
                        },
                    }
                ),
                &Entry::flip_card(OwnedCard::p2(card_points_up), 8, Player::P1, false),
                &Entry::battle(
                    OwnedCard::p1(card_points_vert),
                    OwnedCard::p2(card_points_down),
                    BattleResult {
                        winner: BattleWinner::Attacker,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x1F,
                            roll: 17
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x1F,
                            roll: 18
                        },
                    }
                ),
                &Entry::flip_card(OwnedCard::p2(card_points_down), 0, Player::P1, false),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn reject_input_if_the_choice_isnt_valid() {
        let card_points_down = Card::from_str("0P10", Arrows::DOWN);
        let card_points_up = Card::from_str("0P10", Arrows::UP);
        let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::UP | Arrows::DOWN);

        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_vert), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();
        let res = next(&mut state, &mut log, Input::battle(4));

        assert_eq!(res, Err("Cell 4 is not a valid choice".into()));
    }

    #[test]
    fn continue_offering_choices_when_multiple_battles_are_still_available() {
        let card_points_down = Card::from_str("0P00", Arrows::DOWN);
        let card_points_left = Card::from_str("0P00", Arrows::LEFT);
        let card_points_up = Card::from_str("0P00", Arrows::UP);
        let card_points_all = Card::from_str("FP00", Arrows::ALL);

        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_all), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[5] = Cell::p2_card(card_points_left);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();
        next(&mut state, &mut log, Input::battle(0)).unwrap();

        assert_eq!(
            state.status,
            GameStatus::WaitingBattle {
                attacker_cell: 4,
                choices: vec![(5, card_points_left), (8, card_points_up)]
            }
        );
    }

    #[test]
    fn dont_continue_offering_choices_if_attacker_loses_battle() {
        let card_points_down = Card::from_str("0PF0", Arrows::DOWN);
        let card_points_left = Card::from_str("0P00", Arrows::LEFT);
        let card_points_up = Card::from_str("0P00", Arrows::UP);
        let card_points_all = Card::from_str("0P00", Arrows::ALL);

        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_all), None, None, None, None],
            ..GameState::empty()
        };
        state.board[0] = Cell::p2_card(card_points_down);
        state.board[5] = Cell::p2_card(card_points_left);
        state.board[8] = Cell::p2_card(card_points_up);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 4)).unwrap();
        next(&mut state, &mut log, Input::battle(0)).unwrap();

        assert_eq!(state.status, GameStatus::WaitingPlace);
        assert_eq!(state.board[0], Cell::p2_card(card_points_down));
        assert_eq!(state.board[5], Cell::p2_card(card_points_left));
        assert_eq!(state.board[8], Cell::p2_card(card_points_up));
        assert_eq!(state.board[4], Cell::p2_card(card_points_all));
    }

    #[test]
    fn combo_flip_cards_that_belong_to_opponent_are_pointed_to_by_card_that_loses_battles() {
        let card_points_all = Card::from_str("0P00", Arrows::ALL);
        let card_points_up = Card::from_str("FP00", Arrows::UP);
        let card_points_none = Card::from_str("0P00", Arrows::NONE);
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card_points_up), None, None, None, None],
            ..GameState::empty()
        };
        state.board[5] = Cell::p2_card(card_points_all);
        state.board[1] = Cell::p2_card(card_points_none);
        state.board[4] = Cell::p2_card(card_points_none);
        state.board[6] = Cell::p2_card(card_points_none);

        let mut log = GameLog::new(state.turn);
        next(&mut state, &mut log, Input::place(0, 9)).unwrap();

        assert_eq!(state.board[5], Cell::p1_card(card_points_all));
        assert_eq!(state.board[1], Cell::p1_card(card_points_none));
        assert_eq!(state.board[4], Cell::p1_card(card_points_none));
        assert_eq!(state.board[6], Cell::p1_card(card_points_none));

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_up), 9),
                &Entry::battle(
                    OwnedCard::p1(card_points_up),
                    OwnedCard::p2(card_points_all),
                    BattleResult {
                        winner: BattleWinner::Attacker,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0xFF,
                            roll: 142
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x0F,
                            roll: 15
                        },
                    }
                ),
                &Entry::flip_card(OwnedCard::p2(card_points_all), 5, Player::P1, false),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 1, Player::P1, true),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 6, Player::P1, true),
                &Entry::flip_card(OwnedCard::p2(card_points_none), 4, Player::P1, true),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn game_should_be_over_once_all_cards_have_been_played() {
        let card = Card::from_str("0P00", Arrows::NONE);

        {
            // player 1 wins
            let mut state = GameState {
                turn: Player::P1,
                p1_hand: [Some(card), Some(card), None, None, None],
                p2_hand: [Some(card), None, None, None, None],
                ..GameState::empty()
            };
            let mut log = GameLog::new(state.turn);

            next(&mut state, &mut log, Input::place(0, 0)).unwrap();
            next(&mut state, &mut log, Input::place(0, 1)).unwrap();
            next(&mut state, &mut log, Input::place(1, 2)).unwrap();

            assert_eq!(
                state.status,
                GameStatus::GameOver {
                    winner: Some(Player::P1)
                }
            );
        }

        {
            // player 2 wins
            let mut state = GameState {
                turn: Player::P2,
                p1_hand: [Some(card), None, None, None, None],
                p2_hand: [Some(card), Some(card), None, None, None],
                ..GameState::empty()
            };
            let mut log = GameLog::new(state.turn);

            next(&mut state, &mut log, Input::place(0, 0)).unwrap();
            next(&mut state, &mut log, Input::place(0, 1)).unwrap();
            next(&mut state, &mut log, Input::place(1, 2)).unwrap();

            assert_eq!(
                state.status,
                GameStatus::GameOver {
                    winner: Some(Player::P2)
                }
            );
        }

        {
            // draw
            let mut state = GameState {
                turn: Player::P2,
                p1_hand: [Some(card), None, None, None, None],
                p2_hand: [Some(card), None, None, None, None],
                ..GameState::empty()
            };
            let mut log = GameLog::new(state.turn);

            next(&mut state, &mut log, Input::place(0, 0)).unwrap();
            next(&mut state, &mut log, Input::place(0, 1)).unwrap();

            assert_eq!(state.status, GameStatus::GameOver { winner: None });
        }
    }

    #[cfg(test)]
    mod test_get_attack_stat {
        use super::*;
        use pretty_assertions::assert_eq;

        fn card(stats: &str) -> Card {
            Card::from_str(stats, Arrows::NONE)
        }

        #[test]
        fn physical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("APBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xAF);
        }

        #[test]
        fn magical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("AMBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xAF);
        }

        #[test]
        fn exploit_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("AXBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xAF);
        }

        #[test]
        fn assault_type_attacker_picks_highest_stat() {
            {
                let stat = get_attack_stat(&rng(), card("FA12"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xFF);
            }
            {
                let stat = get_attack_stat(&rng(), card("AAB2"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xBF);
            }
            {
                let stat = get_attack_stat(&rng(), card("AA1F"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xFF);
            }

            // when there is a tie between the attack stat and a defense stat, prefer the attack
            {
                assert_eq!(get_attack_stat(&rng(), card("FAF0")).digit, 0);
                assert_eq!(get_attack_stat(&rng(), card("FA0F")).digit, 0);
                assert_eq!(get_attack_stat(&rng(), card("FAFF")).digit, 0);
            }
        }
    }

    #[cfg(test)]
    mod test_get_defense_stat {
        use super::*;
        use pretty_assertions::assert_eq;

        fn card(stats: &str) -> Card {
            Card::from_str(stats, Arrows::NONE)
        }

        #[test]
        fn physical_type_attacker_picks_physical_defense() {
            let attacker = card("0P00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), attacker, defender);
            assert_eq!(stat.digit, 2);
            assert_eq!(stat.value, 0xBF);
        }

        #[test]
        fn magical_type_attacker_picks_magical_defense() {
            let attacker = card("0M00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), attacker, defender);
            assert_eq!(stat.digit, 3);
            assert_eq!(stat.value, 0xCF);
        }

        #[test]
        fn exploit_type_attacker_picks_lowest_defense() {
            let attacker = card("0X00");
            {
                let stat = get_defense_stat(&rng(), attacker, card("APBC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xBF);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("APCB"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xBF);
            }
        }

        #[test]
        fn assault_type_attacker_picks_lowest_stat() {
            let attacker = card("0A00");
            {
                let stat = get_defense_stat(&rng(), attacker, card("APBC"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xAF);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("BPAC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xAF);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("CPBA"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xAF);
            }
        }
    }
}
