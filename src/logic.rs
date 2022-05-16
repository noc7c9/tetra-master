use crate::{
    Arrows, BattleResult, BattleStat, BattleWinner, Card, CardType, Cell, GameState, Move,
    OwnedCard, Player, {Entry, GameLog},
};

pub(crate) fn next(state: &mut GameState, log: &mut GameLog, input: Move) -> Result<(), String> {
    let hand = match state.turn {
        Player::P1 => &mut state.p1_hand,
        Player::P2 => &mut state.p2_hand,
    };

    // ensure cell being placed is empty
    if !matches!(state.board[input.cell], Cell::Empty) {
        return Err(format!("Cell {:X} is not empty", input.cell));
    }

    // remove the card from the hand
    let mut attacker = match hand[input.card].take() {
        None => {
            return Err(format!("Card {} has already been played", input.card));
        }
        Some(card) => OwnedCard {
            owner: state.turn,
            card,
        },
    };

    // append the place event here to ensure correct ordering
    log.append(Entry::place_card(attacker, input.cell));

    // handle interactions
    for &(idx, arrow) in get_neighbours(input.cell).iter() {
        if let Cell::Card(defender) = &mut state.board[idx] {
            // skip over cards belong to the attacking player
            if defender.owner == attacker.owner {
                continue;
            }

            // skip if the attacking card doesn't have an arrow in this direction
            if !is_attacking(attacker.card, arrow) {
                continue;
            }

            if is_defending(defender.card, arrow) {
                let result = run_battle(&state.rng, attacker.card, defender.card);
                log.append(Entry::battle(attacker, *defender, result));
                match result.winner {
                    BattleWinner::Attacker => {
                        // flip defender
                        log.append(Entry::flip_card(*defender, idx, attacker.owner));
                        defender.owner = attacker.owner;
                    }
                    BattleWinner::Defender | BattleWinner::None => {
                        // flip attacker
                        log.append(Entry::flip_card(attacker, input.cell, defender.owner));
                        attacker.owner = defender.owner;
                    }
                }
            } else {
                // card isn't defending so flip it
                log.append(Entry::flip_card(*defender, idx, attacker.owner));
                defender.owner = attacker.owner;
            }
        }
    }

    // move card onto the board
    state.board[input.cell] = Cell::Card(attacker);

    // next turn
    state.turn = state.turn.opposite();
    log.append(Entry::next_turn(state.turn));

    Ok(())
}

fn is_attacking(card: Card, attack_direction: Arrows) -> bool {
    card.arrows.has(attack_direction)
}

fn is_defending(card: Card, attack_direction: Arrows) -> bool {
    card.arrows.flip().has(attack_direction)
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

fn run_battle(rng: &fastrand::Rng, attacker: Card, defender: Card) -> BattleResult {
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

// returns index of neighbour cells along with the arrow that points at the neighbour
fn get_neighbours(cell: usize) -> &'static [(usize, Arrows)] {
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
    use crate::Arrows;

    fn rng() -> fastrand::Rng {
        fastrand::Rng::new()
    }

    fn with_seed(seed: u64) -> fastrand::Rng {
        fastrand::Rng::with_seed(seed)
    }

    impl GameState {
        fn empty() -> Self {
            GameState {
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
            let attack = u8::from_str_radix(&stats[0..1], 16).unwrap();
            let card_type = match &stats[1..2] {
                "P" => CardType::Physical,
                "M" => CardType::Magical,
                "X" => CardType::Exploit,
                "A" => CardType::Assault,
                _ => unreachable!(),
            };
            let physical_defense = u8::from_str_radix(&stats[2..3], 16).unwrap();
            let magical_defense = u8::from_str_radix(&stats[3..4], 16).unwrap();
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

    #[test]
    fn turn_should_change_after_playing_a_move() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        next(&mut state, &mut log, Move { card: 0, cell: 0 }).unwrap();

        assert_eq!(state.turn, Player::P2);
    }

    #[test]
    fn reject_move_if_the_card_has_already_been_played() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [None, None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        let res = next(&mut state, &mut log, Move { card: 2, cell: 0 });

        assert_eq!(res, Err("Card 2 has already been played".into()));
    }

    #[test]
    fn reject_move_if_the_cell_played_on_is_blocked() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        state.board[0xB] = Cell::Blocked;

        let res = next(&mut state, &mut log, Move { card: 0, cell: 0xB });

        assert_eq!(res, Err("Cell B is not empty".into()));
    }

    #[test]
    fn reject_move_if_the_cell_played_on_already_has_a_card_placed() {
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(Card::basic()), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);
        state.board[3] = Cell::p1_card(Card::basic());

        let res = next(&mut state, &mut log, Move { card: 0, cell: 3 });

        assert_eq!(res, Err("Cell 3 is not empty".into()));
    }

    #[test]
    fn move_card_from_hand_to_board_if_move_is_valid() {
        let card = Card::basic();
        let mut state = GameState {
            turn: Player::P1,
            p1_hand: [Some(card), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new(state.turn);

        next(&mut state, &mut log, Move { card: 0, cell: 7 }).unwrap();

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

        next(&mut state, &mut log, Move { card: 0, cell: 7 }).unwrap();

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

        next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

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

        next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::next_turn(Player::P1),
                &Entry::place_card(OwnedCard::p1(card_points_up), 4,),
                &Entry::flip_card(OwnedCard::p2(card_no_arrows), 0, Player::P1),
                &Entry::next_turn(Player::P2),
            ]
        );
    }

    #[test]
    fn battle_cards_that_belong_to_opponent_are_pointed_to_and_point_back() {
        let card_points_down = Card::from_str("0P90", Arrows::DOWN);
        let card_points_up = Card::from_str("9P00", Arrows::UP);
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
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

            assert_eq!(state.board[0], Cell::p1_card(card_points_down));
            assert_eq!(state.board[4], Cell::p1_card(card_points_up));
        }

        {
            // rng is set to make the defender win
            let mut state = GameState {
                rng: with_seed(1),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

            assert_eq!(state.board[0], Cell::p2_card(card_points_down));
            assert_eq!(state.board[4], Cell::p2_card(card_points_up));
        }

        {
            // rng is set to make the battle draw and default as a defender win
            let mut state = GameState {
                rng: with_seed(5),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

            assert_eq!(state.board[0], Cell::p2_card(card_points_down));
            assert_eq!(state.board[4], Cell::p2_card(card_points_up));
        }
    }

    #[test]
    fn update_game_log_on_battles() {
        let card_points_down = Card::from_str("0P90", Arrows::DOWN);
        let card_points_up = Card::from_str("9P00", Arrows::UP);
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
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

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
                                value: 9,
                                roll: 5
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 9,
                                roll: 9
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p2(card_points_down), 0, Player::P1),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }

        {
            // rng is set to make the defender win
            let mut state = GameState {
                rng: with_seed(1),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

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
                                value: 9,
                                roll: 8
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 9,
                                roll: 1
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p1(card_points_up), 4, Player::P2),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }

        {
            // rng is set to make the battle draw and default as a defender win
            let mut state = GameState {
                rng: with_seed(5),
                ..state
            };
            let mut log = GameLog::new(state.turn);
            next(&mut state, &mut log, Move { card: 0, cell: 4 }).unwrap();

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
                                value: 9,
                                roll: 6
                            },
                            defense_stat: BattleStat {
                                digit: 2,
                                value: 9,
                                roll: 6
                            },
                        }
                    ),
                    &Entry::flip_card(OwnedCard::p1(card_points_up), 4, Player::P2),
                    &Entry::next_turn(Player::P2),
                ]
            );
        }
    }

    #[cfg(test)]
    mod test_get_attack_stat {
        use super::*;

        fn card(stats: &str) -> Card {
            Card::from_str(stats, Arrows::NONE)
        }

        #[test]
        fn physical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("APBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn magical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("AMBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn exploit_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), card("AXBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn assault_type_attacker_picks_highest_stat() {
            {
                let stat = get_attack_stat(&rng(), card("FA12"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xF);
            }
            {
                let stat = get_attack_stat(&rng(), card("AAB2"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xB);
            }
            {
                let stat = get_attack_stat(&rng(), card("AA1F"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xF);
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

        fn card(stats: &str) -> Card {
            Card::from_str(stats, Arrows::NONE)
        }

        #[test]
        fn physical_type_attacker_picks_physical_defense() {
            let attacker = card("0P00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), attacker, defender);
            assert_eq!(stat.digit, 2);
            assert_eq!(stat.value, 0xB);
        }

        #[test]
        fn magical_type_attacker_picks_magical_defense() {
            let attacker = card("0M00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), attacker, defender);
            assert_eq!(stat.digit, 3);
            assert_eq!(stat.value, 0xC);
        }

        #[test]
        fn exploit_type_attacker_picks_lowest_defense() {
            let attacker = card("0X00");
            {
                let stat = get_defense_stat(&rng(), attacker, card("APBC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xB);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("APCB"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xB);
            }
        }

        #[test]
        fn assault_type_attacker_picks_lowest_stat() {
            let attacker = card("0A00");
            {
                let stat = get_defense_stat(&rng(), attacker, card("APBC"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xA);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("BPAC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xA);
            }
            {
                let stat = get_defense_stat(&rng(), attacker, card("CPBA"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xA);
            }
        }
    }
}
