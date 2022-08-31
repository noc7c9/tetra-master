use super::*;
use pretty_assertions::assert_eq;

// board cells references
//
//  0 | 1 | 2 | 3
// ---+---+---+---
//  4 | 5 | 6 | 7
// ---+---+---+---
//  8 | 9 | A | B
// ---+---+---+---
//  C | D | E | F

impl GameState {
    fn empty() -> Self {
        let card = Some(Card::basic());
        GameState {
            status: GameStatus::WaitingPlace,
            rng: Rng::with_seed(0),
            turn: Player::P1,
            board: Default::default(),
            p1_hand: [card, card, card, card, card],
            p2_hand: [card, card, card, card, card],
            battle_system: BattleSystem::Original,
        }
    }
}

impl Card {
    fn from_str(stats: &str, arrows: Arrows) -> Self {
        let card_type = match &stats[1..2] {
            "P" => CardType::Physical,
            "M" => CardType::Magical,
            "X" => CardType::Exploit,
            "A" => CardType::Assault,
            _ => unreachable!(),
        };
        let attack = u8::from_str_radix(&stats[0..1], 16).unwrap();
        let physical_defense = u8::from_str_radix(&stats[2..3], 16).unwrap();
        let magical_defense = u8::from_str_radix(&stats[3..4], 16).unwrap();
        Card::new(attack, card_type, physical_defense, magical_defense, arrows)
    }

    fn basic() -> Self {
        Card::from_str("0P00", Arrows::NONE)
    }

    fn basic_with(arrows: Arrows) -> Self {
        let mut card = Card::basic();
        card.arrows = arrows;
        card
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

impl GameInput {
    fn place(card: usize, cell: usize) -> Self {
        GameInput::Place(GameInputPlace { card, cell })
    }

    fn battle(cell: usize) -> Self {
        GameInput::Battle(GameInputBattle { cell })
    }
}

#[test]
fn turn_should_change_after_a_valid_play() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.turn = Player::P1;
    state.p1_hand[0] = Some(Card::basic());

    game_next(&mut state, &mut log, GameInput::place(0, 0)).unwrap();

    assert_eq!(state.turn, Player::P2);
}

#[test]
fn reject_input_if_the_card_has_already_been_played() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[2] = None;

    let res = game_next(&mut state, &mut log, GameInput::place(2, 0));

    assert_eq!(res, Err("Card 2 has already been played".into()));
}

#[test]
fn reject_input_if_the_cell_played_on_is_blocked() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[0] = Some(Card::basic());
    state.board[0xB] = Cell::Blocked;

    let res = game_next(&mut state, &mut log, GameInput::place(0, 0xB));

    assert_eq!(res, Err("Cell B is not empty".into()));
}

#[test]
fn reject_input_if_the_cell_played_on_already_has_a_card_placed() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[0] = Some(Card::basic());
    state.board[3] = Cell::p1_card(Card::basic());

    let res = game_next(&mut state, &mut log, GameInput::place(0, 3));

    assert_eq!(res, Err("Cell 3 is not empty".into()));
}

#[test]
fn move_card_from_hand_to_board_if_input_is_valid() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card = Card::basic();
    state.p1_hand[0] = Some(card);

    game_next(&mut state, &mut log, GameInput::place(0, 7)).unwrap();

    assert_eq!(state.p1_hand[0], None);
    assert_eq!(state.board[0x7], Cell::p1_card(card));
}

#[test]
fn update_game_log_on_placing_card() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card = Card::basic();
    state.p1_hand[0] = Some(card);

    game_next(&mut state, &mut log, GameInput::place(0, 7)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card), 7),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn flip_cards_that_belong_to_opponent_are_pointed_to_and_dont_point_back() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_no_arrows = Card::basic_with(Arrows::NONE);
    state.p1_hand[0] = Some(Card::basic_with(Arrows::UP | Arrows::RIGHT));
    // should flip, is pointed to, belongs to opponent
    state.board[0] = Cell::p2_card(card_no_arrows);
    // shouldn't flip, doesn't belongs to opponent
    state.board[5] = Cell::p1_card(card_no_arrows);
    // shouldn't flip, isn't pointed to
    state.board[8] = Cell::p2_card(card_no_arrows);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

    assert_eq!(state.board[0], Cell::p1_card(card_no_arrows));
    assert_eq!(state.board[5], Cell::p1_card(card_no_arrows));
    assert_eq!(state.board[8], Cell::p2_card(card_no_arrows));
}

#[test]
fn update_game_log_on_flipping_cards() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_no_arrows = Card::basic_with(Arrows::NONE);
    let card_points_up = Card::basic_with(Arrows::UP | Arrows::RIGHT);
    state.p1_hand[0] = Some(card_points_up);
    state.board[0] = Cell::p2_card(card_no_arrows);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_up), 4),
            &Entry::flip_card(OwnedCard::p2(card_no_arrows), 0, Player::P1, false),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn battle_cards_that_belong_to_opponent_are_pointed_to_and_point_back() {
    let mut state = GameState::empty();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("1P00", Arrows::UP);
    state.p1_hand[0] = Some(card_points_up);
    state.board[0] = Cell::p2_card(card_points_down);

    {
        // rng is set to make the attacker win
        let mut state = GameState {
            rng: Rng::with_seed(1),
            ..state.clone()
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        assert_eq!(state.board[0], Cell::p1_card(card_points_down));
        assert_eq!(state.board[4], Cell::p1_card(card_points_up));
    }

    {
        // rng is set to make the defender win
        let mut state = GameState {
            rng: Rng::with_seed(0),
            ..state.clone()
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        assert_eq!(state.board[0], Cell::p2_card(card_points_down));
        assert_eq!(state.board[4], Cell::p2_card(card_points_up));
    }

    {
        // rng is set to make the battle draw and default as a defender win
        let mut state = GameState {
            rng: Rng::with_seed(94),
            ..state
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        // FIXME this test doesn't assert that the result was a draw
        assert_eq!(state.board[0], Cell::p2_card(card_points_down));
        assert_eq!(state.board[4], Cell::p2_card(card_points_up));
    }
}

#[test]
fn update_game_log_on_battles() {
    let mut state = GameState::empty();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("1P00", Arrows::UP);
    state.p1_hand[0] = Some(card_points_up);
    state.board[0] = Cell::p2_card(card_points_down);

    {
        // rng is set to make the attacker win
        let mut state = GameState {
            rng: Rng::with_seed(1),
            ..state.clone()
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_up),
                    4,
                    OwnedCard::p2(card_points_down),
                    0,
                    BattleResult {
                        winner: BattleWinner::Attacker,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x1,
                            roll: 27
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x1,
                            roll: 0
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
            rng: Rng::with_seed(0),
            ..state.clone()
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_up),
                    4,
                    OwnedCard::p2(card_points_down),
                    0,
                    BattleResult {
                        winner: BattleWinner::Defender,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x1,
                            roll: 0,
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x1,
                            roll: 10,
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
            rng: Rng::with_seed(44),
            ..state
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_up),
                    4,
                    OwnedCard::p2(card_points_down),
                    0,
                    BattleResult {
                        winner: BattleWinner::None,
                        attack_stat: BattleStat {
                            digit: 0,
                            value: 0x1,
                            roll: 4
                        },
                        defense_stat: BattleStat {
                            digit: 2,
                            value: 0x1,
                            roll: 4
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
    let mut state = GameState {
        rng: Rng::with_seed(1),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    let card_points_down = Card::from_str("0P00", Arrows::DOWN);
    let card_points_all = Card::from_str("FP00", Arrows::ALL);
    state.p1_hand[0] = Some(card_points_all);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[1] = Cell::p2_card(card_points_none);
    state.board[5] = Cell::p2_card(card_points_none);
    state.board[9] = Cell::p2_card(card_points_none);

    // placed card attacks card above (0), wins and flips the other cards (1, 5, 9, 4)
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

    assert_eq!(state.board[0], Cell::p1_card(card_points_down));
    assert_eq!(state.board[1], Cell::p1_card(card_points_none));
    assert_eq!(state.board[5], Cell::p1_card(card_points_none));
    assert_eq!(state.board[9], Cell::p1_card(card_points_none));
    assert_eq!(state.board[4], Cell::p1_card(card_points_all));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_all), 4),
            &Entry::battle(
                OwnedCard::p1(card_points_all),
                4,
                OwnedCard::p2(card_points_down),
                0,
                BattleResult {
                    winner: BattleWinner::Attacker,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0xF,
                        roll: 226
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0x0,
                        roll: 0
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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    let card_points_down = Card::from_str("0PF0", Arrows::DOWN);
    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    state.p1_hand[0] = Some(card_points_all);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[1] = Cell::p2_card(card_points_none);
    state.board[5] = Cell::p2_card(card_points_none);
    state.board[9] = Cell::p2_card(card_points_none);

    // placed card attacks card above (0), loses so other cards aren't flipped
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

    assert_eq!(state.board[0], Cell::p2_card(card_points_down));
    assert_eq!(state.board[1], Cell::p2_card(card_points_none));
    assert_eq!(state.board[5], Cell::p2_card(card_points_none));
    assert_eq!(state.board[9], Cell::p2_card(card_points_none));
    assert_eq!(state.board[4], Cell::p2_card(card_points_all));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_all), 4),
            &Entry::battle(
                OwnedCard::p1(card_points_all),
                4,
                OwnedCard::p2(card_points_down),
                0,
                BattleResult {
                    winner: BattleWinner::Defender,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0x0,
                        roll: 0
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0xF,
                        roll: 107
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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![&Entry::place_card(OwnedCard::p1(card_points_vert), 4),]
    );
}

#[test]
fn continue_after_battle_choice_is_given() {
    let mut state = GameState {
        rng: Rng::with_seed(2),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("3P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    // placed card attacks both 0 and 8
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    // attack card 8
    game_next(&mut state, &mut log, GameInput::battle(8)).unwrap();

    // all attacked cards will be flipped
    assert_eq!(state.board[0], Cell::p1_card(card_points_down));
    assert_eq!(state.board[4], Cell::p1_card(card_points_vert));
    assert_eq!(state.board[8], Cell::p1_card(card_points_up));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_vert), 4),
            &Entry::battle(
                OwnedCard::p1(card_points_vert),
                4,
                OwnedCard::p2(card_points_up),
                8,
                BattleResult {
                    winner: BattleWinner::Attacker,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0x3,
                        roll: 38
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0x1,
                        roll: 13
                    },
                }
            ),
            &Entry::flip_card(OwnedCard::p2(card_points_up), 8, Player::P1, false),
            &Entry::battle(
                OwnedCard::p1(card_points_vert),
                4,
                OwnedCard::p2(card_points_down),
                0,
                BattleResult {
                    winner: BattleWinner::Attacker,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0x3,
                        roll: 47
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0x1,
                        roll: 0
                    },
                }
            ),
            &Entry::flip_card(OwnedCard::p2(card_points_down), 0, Player::P1, false),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn change_turn_after_choice_battle_if_attacker_wins() {
    let mut state = GameState {
        rng: Rng::with_seed(2),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("3P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    // placed card attacks both 0 and 8
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    // attack card 8
    game_next(&mut state, &mut log, GameInput::battle(8)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log[1],
        &Entry::battle(
            OwnedCard::p1(card_points_vert),
            4,
            OwnedCard::p2(card_points_up),
            8,
            BattleResult {
                winner: BattleWinner::Attacker,
                attack_stat: BattleStat {
                    digit: 0,
                    value: 3,
                    roll: 38
                },
                defense_stat: BattleStat {
                    digit: 2,
                    value: 1,
                    roll: 13
                },
            }
        )
    );
    assert_eq!(state.turn, Player::P2);
    assert_eq!(log[5], &Entry::next_turn(Player::P2));
}

#[test]
fn change_turn_after_choice_battle_if_defender_wins() {
    let mut state = GameState {
        rng: Rng::with_seed(0),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("3P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    // placed card attacks both 0 and 8
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    // attack card 8
    game_next(&mut state, &mut log, GameInput::battle(8)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log[1],
        &Entry::battle(
            OwnedCard::p1(card_points_vert),
            4,
            OwnedCard::p2(card_points_up),
            8,
            BattleResult {
                winner: BattleWinner::Defender,
                attack_stat: BattleStat {
                    digit: 0,
                    value: 3,
                    roll: 0
                },
                defense_stat: BattleStat {
                    digit: 2,
                    value: 1,
                    roll: 10
                },
            }
        )
    );
    assert_eq!(state.turn, Player::P2);
    assert_eq!(log[3], &Entry::next_turn(Player::P2));
}

#[test]
fn change_turn_after_choice_battle_if_its_a_draw() {
    let mut state = GameState {
        rng: Rng::with_seed(25),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("3P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    // placed card attacks both 0 and 8
    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    // attack card 8
    game_next(&mut state, &mut log, GameInput::battle(8)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log[1],
        &Entry::battle(
            OwnedCard::p1(card_points_vert),
            4,
            OwnedCard::p2(card_points_up),
            8,
            BattleResult {
                winner: BattleWinner::None,
                attack_stat: BattleStat {
                    digit: 0,
                    value: 3,
                    roll: 24
                },
                defense_stat: BattleStat {
                    digit: 2,
                    value: 1,
                    roll: 24
                },
            }
        )
    );
    assert_eq!(state.turn, Player::P2);
    assert_eq!(log[3], &Entry::next_turn(Player::P2));
}

#[test]
fn reject_input_if_the_choice_isnt_valid() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    let res = game_next(&mut state, &mut log, GameInput::battle(4));

    assert_eq!(res, Err("Cell 4 is not a valid choice".into()));
}

#[test]
fn continue_offering_choices_when_multiple_battles_are_still_available() {
    let mut state = GameState {
        rng: Rng::with_seed(1),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P00", Arrows::DOWN);
    let card_points_left = Card::from_str("0P00", Arrows::LEFT);
    let card_points_up = Card::from_str("0P00", Arrows::UP);
    let card_points_all = Card::from_str("FP00", Arrows::ALL);
    state.p1_hand[0] = Some(card_points_all);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[5] = Cell::p2_card(card_points_left);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    game_next(&mut state, &mut log, GameInput::battle(0)).unwrap();

    assert_eq!(state.turn, Player::P1);
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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0PF0", Arrows::DOWN);
    let card_points_left = Card::from_str("0P00", Arrows::LEFT);
    let card_points_up = Card::from_str("0P00", Arrows::UP);
    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    state.p1_hand[0] = Some(card_points_all);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[5] = Cell::p2_card(card_points_left);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    game_next(&mut state, &mut log, GameInput::battle(0)).unwrap();

    assert_eq!(state.status, GameStatus::WaitingPlace);
    assert_eq!(state.board[0], Cell::p2_card(card_points_down));
    assert_eq!(state.board[5], Cell::p2_card(card_points_left));
    assert_eq!(state.board[8], Cell::p2_card(card_points_up));
    assert_eq!(state.board[4], Cell::p2_card(card_points_all));
}

#[test]
fn handle_game_over_when_attacker_loses_battle_after_battle_choice() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0PF0", Arrows::DOWN);
    let card_points_left = Card::from_str("0P00", Arrows::LEFT);
    let card_points_up = Card::from_str("0P00", Arrows::UP);
    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    state.p1_hand = [Some(card_points_all), None, None, None, None];
    state.p2_hand = [None, None, None, None, None];
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[5] = Cell::p2_card(card_points_left);
    state.board[8] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 4)).unwrap();
    game_next(&mut state, &mut log, GameInput::battle(0)).unwrap();

    assert_eq!(
        state.status,
        GameStatus::GameOver {
            winner: Some(Player::P2)
        }
    );
    assert_eq!(state.board[0], Cell::p2_card(card_points_down));
    assert_eq!(state.board[5], Cell::p2_card(card_points_left));
    assert_eq!(state.board[8], Cell::p2_card(card_points_up));
    assert_eq!(state.board[4], Cell::p2_card(card_points_all));
}

#[test]
fn combo_flip_cards_that_are_pointed_to_by_defender_if_they_lose() {
    let mut state = GameState {
        rng: Rng::with_seed(1),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    let card_points_up = Card::from_str("FP00", Arrows::UP);
    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    state.p1_hand[0] = Some(card_points_up);
    state.board[5] = Cell::p2_card(card_points_all);
    state.board[1] = Cell::p2_card(card_points_none);
    state.board[4] = Cell::p2_card(card_points_none);
    state.board[6] = Cell::p2_card(card_points_none);

    game_next(&mut state, &mut log, GameInput::place(0, 9)).unwrap();

    assert_eq!(state.board[5], Cell::p1_card(card_points_all));
    assert_eq!(state.board[1], Cell::p1_card(card_points_none));
    assert_eq!(state.board[4], Cell::p1_card(card_points_none));
    assert_eq!(state.board[6], Cell::p1_card(card_points_none));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_up), 9),
            &Entry::battle(
                OwnedCard::p1(card_points_up),
                9,
                OwnedCard::p2(card_points_all),
                5,
                BattleResult {
                    winner: BattleWinner::Attacker,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0xF,
                        roll: 226
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0x0,
                        roll: 0
                    },
                }
            ),
            &Entry::flip_card(OwnedCard::p2(card_points_all), 5, Player::P1, false),
            &Entry::flip_card(OwnedCard::p2(card_points_none), 1, Player::P1, true),
            &Entry::flip_card(OwnedCard::p2(card_points_none), 4, Player::P1, true),
            &Entry::flip_card(OwnedCard::p2(card_points_none), 6, Player::P1, true),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn combo_flip_cards_that_are_pointed_to_by_attacker_if_they_lose() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    let card_points_up = Card::from_str("0PF0", Arrows::UP);
    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    state.p1_hand[0] = Some(card_points_all);
    state.board[1] = Cell::p1_card(card_points_none);
    state.board[4] = Cell::p1_card(card_points_none);
    state.board[6] = Cell::p1_card(card_points_none);
    state.board[9] = Cell::p2_card(card_points_up);

    game_next(&mut state, &mut log, GameInput::place(0, 5)).unwrap();

    assert_eq!(state.board[5], Cell::p2_card(card_points_all));
    assert_eq!(state.board[1], Cell::p2_card(card_points_none));
    assert_eq!(state.board[4], Cell::p2_card(card_points_none));
    assert_eq!(state.board[6], Cell::p2_card(card_points_none));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_all), 5),
            &Entry::battle(
                OwnedCard::p1(card_points_all),
                5,
                OwnedCard::p2(card_points_up),
                9,
                BattleResult {
                    winner: BattleWinner::Defender,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0x0,
                        roll: 0
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0xF,
                        roll: 107
                    },
                }
            ),
            &Entry::flip_card(OwnedCard::p1(card_points_all), 5, Player::P2, false),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 1, Player::P2, true),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 4, Player::P2, true),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 6, Player::P2, true),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn dont_flip_back_undefended_cards_if_they_are_flipped_due_to_combos() {
    let mut state = GameState {
        rng: Rng::with_seed(1),
        ..GameState::empty()
    };
    let mut log = GameLog::new();

    let card_points_all_att = Card::from_str("FP00", Arrows::ALL);
    let card_points_all_def = Card::from_str("0P00", Arrows::ALL);
    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    state.p1_hand[0] = Some(card_points_all_att);
    state.board[0] = Cell::p2_card(card_points_all_def);
    state.board[4] = Cell::p2_card(card_points_none);

    // placed card points to both other cards, attacker wins, and card on 4 get's combo flipped
    game_next(&mut state, &mut log, GameInput::place(0, 5)).unwrap();

    // all cards should be owned by player 1
    assert_eq!(state.board[5], Cell::p1_card(card_points_all_att));
    assert_eq!(state.board[0], Cell::p1_card(card_points_all_def));
    assert_eq!(state.board[4], Cell::p1_card(card_points_none));

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![
            &Entry::place_card(OwnedCard::p1(card_points_all_att), 5),
            &Entry::battle(
                OwnedCard::p1(card_points_all_att),
                5,
                OwnedCard::p2(card_points_all_def),
                0,
                BattleResult {
                    winner: BattleWinner::Attacker,
                    attack_stat: BattleStat {
                        digit: 0,
                        value: 0xF,
                        roll: 226
                    },
                    defense_stat: BattleStat {
                        digit: 2,
                        value: 0x0,
                        roll: 0
                    },
                }
            ),
            &Entry::flip_card(OwnedCard::p2(card_points_all_def), 0, Player::P1, false),
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
        let mut log = GameLog::new();

        state.p1_hand[0] = Some(card);
        state.p1_hand[1] = Some(card);
        state.p2_hand[0] = Some(card);

        game_next(&mut state, &mut log, GameInput::place(0, 0)).unwrap();
        game_next(&mut state, &mut log, GameInput::place(0, 1)).unwrap();
        game_next(&mut state, &mut log, GameInput::place(1, 2)).unwrap();

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
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 0)).unwrap();
        game_next(&mut state, &mut log, GameInput::place(0, 1)).unwrap();
        game_next(&mut state, &mut log, GameInput::place(1, 2)).unwrap();

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
            p1_hand: [Some(card), None, None, None, None],
            p2_hand: [Some(card), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new();

        game_next(&mut state, &mut log, GameInput::place(0, 0)).unwrap();
        game_next(&mut state, &mut log, GameInput::place(0, 1)).unwrap();

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
        let mut bs = BattleSystem::Original;
        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("APBC"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xA);
    }

    #[test]
    fn magical_type_attacker_picks_attack_stat() {
        let mut bs = BattleSystem::Original;
        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("AMBC"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xA);
    }

    #[test]
    fn exploit_type_attacker_picks_attack_stat() {
        let mut bs = BattleSystem::Original;
        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("AXBC"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xA);
    }

    #[test]
    fn assault_type_attacker_picks_highest_stat() {
        let mut bs = BattleSystem::Original;
        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("FA12"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xF);

        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("AAB2"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xB);

        let stat = get_attack_stat(&mut Rng::new(), &mut bs, card("AA1F"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xF);

        // when there is a tie between the attack stat and a defense stat, prefer the attack
        assert_eq!(
            get_attack_stat(&mut Rng::new(), &mut bs, card("FAF0")).digit,
            0
        );
        assert_eq!(
            get_attack_stat(&mut Rng::new(), &mut bs, card("FA0F")).digit,
            0
        );
        assert_eq!(
            get_attack_stat(&mut Rng::new(), &mut bs, card("FAFF")).digit,
            0
        );
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
        let mut bs = BattleSystem::Original;
        let attacker = card("0P00");
        let defender = card("APBC");
        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, defender);
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xB);
    }

    #[test]
    fn magical_type_attacker_picks_magical_defense() {
        let mut bs = BattleSystem::Original;
        let attacker = card("0M00");
        let defender = card("APBC");
        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, defender);
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xC);
    }

    #[test]
    fn exploit_type_attacker_picks_lowest_defense() {
        let mut bs = BattleSystem::Original;
        let attacker = card("0X00");

        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, card("APBC"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xB);

        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, card("APCB"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xB);
    }

    #[test]
    fn assault_type_attacker_picks_lowest_stat() {
        let mut bs = BattleSystem::Original;
        let attacker = card("0A00");

        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, card("APBC"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xA);

        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, card("BPAC"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xA);

        let stat = get_defense_stat(&mut Rng::new(), &mut bs, attacker, card("CPBA"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xA);
    }
}
