use crate::{
    BattleResult, BattleStat, BattleWinner, Card, CardType, Cell, GameState, Move, OwnedCard,
    Player, {Entry, GameLog},
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
            if !is_attacking(&attacker.card, arrow) {
                continue;
            }

            if is_defending(&defender.card, arrow) {
                let result = run_battle(&state.rng, &attacker.card, &defender.card);
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

fn is_attacking(attacking_card: &Card, attack_direction: Arrow) -> bool {
    match attack_direction {
        Arrow::TopLeft => attacking_card.arrows.top_left,
        Arrow::Top => attacking_card.arrows.top,
        Arrow::TopRight => attacking_card.arrows.top_right,
        Arrow::Left => attacking_card.arrows.left,
        Arrow::Right => attacking_card.arrows.right,
        Arrow::BottomLeft => attacking_card.arrows.bottom_left,
        Arrow::Bottom => attacking_card.arrows.bottom,
        Arrow::BottomRight => attacking_card.arrows.bottom_right,
    }
}

fn is_defending(attacked_card: &Card, attack_direction: Arrow) -> bool {
    match attack_direction {
        Arrow::TopLeft => attacked_card.arrows.bottom_right,
        Arrow::Top => attacked_card.arrows.bottom,
        Arrow::TopRight => attacked_card.arrows.bottom_left,
        Arrow::Left => attacked_card.arrows.right,
        Arrow::Right => attacked_card.arrows.left,
        Arrow::BottomLeft => attacked_card.arrows.top_right,
        Arrow::Bottom => attacked_card.arrows.top,
        Arrow::BottomRight => attacked_card.arrows.top_left,
    }
}

fn get_attack_stat(rng: &fastrand::Rng, attacker: &Card) -> BattleStat {
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

fn get_defense_stat(rng: &fastrand::Rng, attacker: &Card, defender: &Card) -> BattleStat {
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

fn run_battle(rng: &fastrand::Rng, attacker: &Card, defender: &Card) -> BattleResult {
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

#[derive(Debug, Clone, Copy)]
enum Arrow {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

// returns index of neighbour cells along with the arrow that points at the neighbour
fn get_neighbours(cell: usize) -> &'static [(usize, Arrow)] {
    use Arrow::*;
    match cell {
        0x0 => &[(0x1, Right), (0x5, BottomRight), (0x4, Bottom)],
        0x1 => &[
            (0x2, Right),
            (0x6, BottomRight),
            (0x5, Bottom),
            (0x4, BottomLeft),
            (0x0, Left),
        ],
        0x2 => &[
            (0x3, Right),
            (0x7, BottomRight),
            (0x6, Bottom),
            (0x5, BottomLeft),
            (0x1, Left),
        ],
        0x3 => &[(0x7, Bottom), (0x6, BottomLeft), (0x2, Left)],
        0x4 => &[
            (0x0, Top),
            (0x1, TopRight),
            (0x5, Right),
            (0x9, BottomRight),
            (0x8, Bottom),
        ],
        0x5 => &[
            (0x1, Top),
            (0x2, TopRight),
            (0x6, Right),
            (0xA, BottomRight),
            (0x9, Bottom),
            (0x8, BottomLeft),
            (0x4, Left),
            (0x0, TopLeft),
        ],
        0x6 => &[
            (0x2, Top),
            (0x3, TopRight),
            (0x7, Right),
            (0xB, BottomRight),
            (0xA, Bottom),
            (0x9, BottomLeft),
            (0x5, Left),
            (0x1, TopLeft),
        ],
        0x7 => &[
            (0x3, Top),
            (0xB, Bottom),
            (0xA, BottomLeft),
            (0x6, Left),
            (0x2, TopLeft),
        ],
        0x8 => &[
            (0x4, Top),
            (0x5, TopRight),
            (0x9, Right),
            (0xD, BottomRight),
            (0xC, Bottom),
        ],
        0x9 => &[
            (0x5, Top),
            (0x6, TopRight),
            (0xA, Right),
            (0xE, BottomRight),
            (0xD, Bottom),
            (0xC, BottomLeft),
            (0x8, Left),
            (0x4, TopLeft),
        ],
        0xA => &[
            (0x6, Top),
            (0x7, TopRight),
            (0xB, Right),
            (0xF, BottomRight),
            (0xE, Bottom),
            (0xD, BottomLeft),
            (0x9, Left),
            (0x5, TopLeft),
        ],
        0xB => &[
            (0x7, Top),
            (0xF, Bottom),
            (0xE, BottomLeft),
            (0xA, Left),
            (0x6, TopLeft),
        ],
        0xC => &[(0x8, Top), (0x9, TopRight), (0xD, Right)],
        0xD => &[
            (0x9, Top),
            (0xA, TopRight),
            (0xE, Right),
            (0xC, Left),
            (0x8, TopLeft),
        ],
        0xE => &[
            (0xA, Top),
            (0xB, TopRight),
            (0xF, Right),
            (0xD, Left),
            (0x9, TopLeft),
        ],
        0xF => &[(0xB, Top), (0xE, Left), (0xA, TopLeft)],
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
                arrows: Arrows::none(),
                attack: 0,
                physical_defense: 0,
                magical_defense: 0,
            }
        }
    }

    impl Arrows {
        fn none() -> Self {
            Arrows {
                top_left: false,
                top: false,
                top_right: false,
                left: false,
                right: false,
                bottom_left: false,
                bottom: false,
                bottom_right: false,
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
            arrows: Arrows::none(),
            ..Card::basic()
        };
        let card_points_up_and_right = Card {
            arrows: Arrows {
                top: true,
                right: true,
                ..Arrows::none()
            },
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
            arrows: Arrows::none(),
            ..Card::basic()
        };
        let card_points_up = Card {
            arrows: Arrows {
                top: true,
                right: true,
                ..Arrows::none()
            },
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
        let card_points_down = Card::from_str(
            "0P90",
            Arrows {
                bottom: true,
                ..Arrows::none()
            },
        );
        let card_points_up = Card::from_str(
            "9P00",
            Arrows {
                top: true,
                ..Arrows::none()
            },
        );
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
        let card_points_down = Card::from_str(
            "0P90",
            Arrows {
                bottom: true,
                ..Arrows::none()
            },
        );
        let card_points_up = Card::from_str(
            "9P00",
            Arrows {
                top: true,
                ..Arrows::none()
            },
        );
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
            Card::from_str(stats, Arrows::none())
        }

        #[test]
        fn physical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), &card("APBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn magical_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), &card("AMBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn exploit_type_attacker_picks_attack_stat() {
            let stat = get_attack_stat(&rng(), &card("AXBC"));
            assert_eq!(stat.digit, 0);
            assert_eq!(stat.value, 0xA);
        }

        #[test]
        fn assault_type_attacker_picks_highest_stat() {
            {
                let stat = get_attack_stat(&rng(), &card("FA12"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xF);
            }
            {
                let stat = get_attack_stat(&rng(), &card("AAB2"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xB);
            }
            {
                let stat = get_attack_stat(&rng(), &card("AA1F"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xF);
            }

            // when there is a tie between the attack stat and a defense stat, prefer the attack
            {
                assert_eq!(get_attack_stat(&rng(), &card("FAF0")).digit, 0);
                assert_eq!(get_attack_stat(&rng(), &card("FA0F")).digit, 0);
                assert_eq!(get_attack_stat(&rng(), &card("FAFF")).digit, 0);
            }
        }
    }

    #[cfg(test)]
    mod test_get_defense_stat {
        use super::*;

        fn card(stats: &str) -> Card {
            Card::from_str(stats, Arrows::none())
        }

        #[test]
        fn physical_type_attacker_picks_physical_defense() {
            let attacker = card("0P00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), &attacker, &defender);
            assert_eq!(stat.digit, 2);
            assert_eq!(stat.value, 0xB);
        }

        #[test]
        fn magical_type_attacker_picks_magical_defense() {
            let attacker = card("0M00");
            let defender = card("APBC");
            let stat = get_defense_stat(&fastrand::Rng::new(), &attacker, &defender);
            assert_eq!(stat.digit, 3);
            assert_eq!(stat.value, 0xC);
        }

        #[test]
        fn exploit_type_attacker_picks_lowest_defense() {
            let attacker = card("0X00");
            {
                let stat = get_defense_stat(&rng(), &attacker, &card("APBC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xB);
            }
            {
                let stat = get_defense_stat(&rng(), &attacker, &card("APCB"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xB);
            }
        }

        #[test]
        fn assault_type_attacker_picks_lowest_stat() {
            let attacker = card("0A00");
            {
                let stat = get_defense_stat(&rng(), &attacker, &card("APBC"));
                assert_eq!(stat.digit, 0);
                assert_eq!(stat.value, 0xA);
            }
            {
                let stat = get_defense_stat(&rng(), &attacker, &card("BPAC"));
                assert_eq!(stat.digit, 2);
                assert_eq!(stat.value, 0xA);
            }
            {
                let stat = get_defense_stat(&rng(), &attacker, &card("CPBA"));
                assert_eq!(stat.digit, 3);
                assert_eq!(stat.value, 0xA);
            }
        }
    }
}
