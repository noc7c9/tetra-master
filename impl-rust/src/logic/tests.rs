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
