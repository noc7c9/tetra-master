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

fn rng() -> fastrand::Rng {
    fastrand::Rng::new()
}

fn with_seed(seed: u64) -> fastrand::Rng {
    fastrand::Rng::with_seed(seed)
}

impl GameState {
    fn empty() -> Self {
        let card = Some(Card::basic());
        GameState {
            status: GameStatus::WaitingPlace,
            rng: fastrand::Rng::with_seed(0),
            turn: Player::P1,
            board: Default::default(),
            p1_hand: [card, card, card, card, card],
            p2_hand: [card, card, card, card, card],
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
        let attack = 0xF + 0x10 * u8::from_str_radix(&stats[0..1], 16).unwrap();
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
        Card::from_str("0P00", Arrows::NONE)
    }

    fn basic_with(arrows: Arrows) -> Self {
        Card {
            arrows,
            ..Card::basic()
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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.turn = Player::P1;
    state.p1_hand[0] = Some(Card::basic());

    next(&mut state, &mut log, Input::place(0, 0)).unwrap();

    assert_eq!(state.turn, Player::P2);
}

#[test]
fn reject_input_if_the_card_has_already_been_played() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[2] = None;

    let res = next(&mut state, &mut log, Input::place(2, 0));

    assert_eq!(res, Err("Card 2 has already been played".into()));
}

#[test]
fn reject_input_if_the_cell_played_on_is_blocked() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[0] = Some(Card::basic());
    state.board[0xB] = Cell::Blocked;

    let res = next(&mut state, &mut log, Input::place(0, 0xB));

    assert_eq!(res, Err("Cell B is not empty".into()));
}

#[test]
fn reject_input_if_the_cell_played_on_already_has_a_card_placed() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    state.p1_hand[0] = Some(Card::basic());
    state.board[3] = Cell::p1_card(Card::basic());

    let res = next(&mut state, &mut log, Input::place(0, 3));

    assert_eq!(res, Err("Cell 3 is not empty".into()));
}

#[test]
fn move_card_from_hand_to_board_if_input_is_valid() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card = Card::basic();
    state.p1_hand[0] = Some(card);

    next(&mut state, &mut log, Input::place(0, 7)).unwrap();

    assert_eq!(state.p1_hand[0], None);
    assert_eq!(state.board[0x7], Cell::p1_card(card));
}

#[test]
fn update_game_log_on_placing_card() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card = Card::basic();
    state.p1_hand[0] = Some(card);

    next(&mut state, &mut log, Input::place(0, 7)).unwrap();

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

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();

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

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();

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
            rng: with_seed(0),
            ..state.clone()
        };
        let mut log = GameLog::new();

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
        let mut log = GameLog::new();

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
        let mut log = GameLog::new();

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

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
            rng: with_seed(0),
            ..state.clone()
        };
        let mut log = GameLog::new();

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
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
        let mut log = GameLog::new();

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
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
        let mut log = GameLog::new();

        next(&mut state, &mut log, Input::place(0, 4)).unwrap();

        let log: Vec<_> = log.iter().collect();
        assert_eq!(
            log,
            vec![
                &Entry::place_card(OwnedCard::p1(card_points_up), 4),
                &Entry::battle(
                    OwnedCard::p1(card_points_up),
                    OwnedCard::p2(card_points_down),
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
    let mut state = GameState::empty();
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
            &Entry::place_card(OwnedCard::p1(card_points_all), 4),
            &Entry::battle(
                OwnedCard::p1(card_points_all),
                OwnedCard::p2(card_points_down),
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
            &Entry::place_card(OwnedCard::p1(card_points_all), 4),
            &Entry::battle(
                OwnedCard::p1(card_points_all),
                OwnedCard::p2(card_points_down),
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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();

    let log: Vec<_> = log.iter().collect();
    assert_eq!(
        log,
        vec![&Entry::place_card(OwnedCard::p1(card_points_vert), 4),]
    );
}

#[test]
fn continue_after_battle_choice_is_given() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    // placed card attacks both 0 and 8
    next(&mut state, &mut log, Input::place(0, 4)).unwrap();
    // attack card 8
    next(&mut state, &mut log, Input::battle(8)).unwrap();

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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P10", Arrows::DOWN);
    let card_points_up = Card::from_str("0P10", Arrows::UP);
    let card_points_vert = Card::from_str("1P00", Arrows::UP | Arrows::DOWN);
    state.p1_hand[0] = Some(card_points_vert);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[8] = Cell::p2_card(card_points_up);

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();
    let res = next(&mut state, &mut log, Input::battle(4));

    assert_eq!(res, Err("Cell 4 is not a valid choice".into()));
}

#[test]
fn continue_offering_choices_when_multiple_battles_are_still_available() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_down = Card::from_str("0P00", Arrows::DOWN);
    let card_points_left = Card::from_str("0P00", Arrows::LEFT);
    let card_points_up = Card::from_str("0P00", Arrows::UP);
    let card_points_all = Card::from_str("FP00", Arrows::ALL);
    state.p1_hand[0] = Some(card_points_all);
    state.board[0] = Cell::p2_card(card_points_down);
    state.board[5] = Cell::p2_card(card_points_left);
    state.board[8] = Cell::p2_card(card_points_up);

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

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();
    next(&mut state, &mut log, Input::battle(0)).unwrap();

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

    next(&mut state, &mut log, Input::place(0, 4)).unwrap();
    next(&mut state, &mut log, Input::battle(0)).unwrap();

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
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_all = Card::from_str("0P00", Arrows::ALL);
    let card_points_up = Card::from_str("FP00", Arrows::UP);
    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    state.p1_hand[0] = Some(card_points_up);
    state.board[5] = Cell::p2_card(card_points_all);
    state.board[1] = Cell::p2_card(card_points_none);
    state.board[4] = Cell::p2_card(card_points_none);
    state.board[6] = Cell::p2_card(card_points_none);

    next(&mut state, &mut log, Input::place(0, 9)).unwrap();

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

    next(&mut state, &mut log, Input::place(0, 5)).unwrap();

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
                OwnedCard::p2(card_points_up),
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
            &Entry::flip_card(OwnedCard::p1(card_points_all), 5, Player::P2, false),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 1, Player::P2, true),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 6, Player::P2, true),
            &Entry::flip_card(OwnedCard::p1(card_points_none), 4, Player::P2, true),
            &Entry::next_turn(Player::P2),
        ]
    );
}

#[test]
fn dont_flip_back_undefended_cards_if_they_are_flipped_due_to_combos() {
    let mut state = GameState::empty();
    let mut log = GameLog::new();

    let card_points_all_att = Card::from_str("FP00", Arrows::ALL);
    let card_points_all_def = Card::from_str("0P00", Arrows::ALL);
    let card_points_none = Card::from_str("0P00", Arrows::NONE);
    state.p1_hand[0] = Some(card_points_all_att);
    state.board[0] = Cell::p2_card(card_points_all_def);
    state.board[4] = Cell::p2_card(card_points_none);

    // placed card points to both other cards, attacker wins, and card on 4 get's combo flipped
    next(&mut state, &mut log, Input::place(0, 5)).unwrap();

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
                OwnedCard::p2(card_points_all_def),
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
        let mut log = GameLog::new();

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
            p1_hand: [Some(card), None, None, None, None],
            p2_hand: [Some(card), None, None, None, None],
            ..GameState::empty()
        };
        let mut log = GameLog::new();

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
        let stat = get_attack_stat(&rng(), card("FA12"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xFF);

        let stat = get_attack_stat(&rng(), card("AAB2"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xBF);

        let stat = get_attack_stat(&rng(), card("AA1F"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xFF);

        // when there is a tie between the attack stat and a defense stat, prefer the attack
        assert_eq!(get_attack_stat(&rng(), card("FAF0")).digit, 0);
        assert_eq!(get_attack_stat(&rng(), card("FA0F")).digit, 0);
        assert_eq!(get_attack_stat(&rng(), card("FAFF")).digit, 0);
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

        let stat = get_defense_stat(&rng(), attacker, card("APBC"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xBF);

        let stat = get_defense_stat(&rng(), attacker, card("APCB"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xBF);
    }

    #[test]
    fn assault_type_attacker_picks_lowest_stat() {
        let attacker = card("0A00");

        let stat = get_defense_stat(&rng(), attacker, card("APBC"));
        assert_eq!(stat.digit, 0);
        assert_eq!(stat.value, 0xAF);

        let stat = get_defense_stat(&rng(), attacker, card("BPAC"));
        assert_eq!(stat.digit, 2);
        assert_eq!(stat.value, 0xAF);

        let stat = get_defense_stat(&rng(), attacker, card("CPBA"));
        assert_eq!(stat.digit, 3);
        assert_eq!(stat.value, 0xAF);
    }
}
