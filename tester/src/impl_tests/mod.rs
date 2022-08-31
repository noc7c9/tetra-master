use pretty_assertions::assert_eq;

use crate::{
    driver::{BattleWinner, Battler, Command, Digit, Event, ImplementationDriver, Response},
    test_harness::Harness,
    Arrows, BattleSystem, Card, Player,
};

mod helpers;

pub(crate) fn run(implementation: String) {
    use helpers::*;

    let new_driver = || ImplementationDriver::new(&implementation);

    let mut harness = Harness::new();

    macro_rules! test {
        ($method:ident; $name:literal $test:block) => {{
            harness.$method($name, || {
                $test;
                Ok(())
            })
        }};
        ($name:literal $test:block) => { test!(test; $name $test) };
    }
    #[allow(unused_macros)]
    macro_rules! focustest {
        ($name:literal $test:block) => { test!(test_focus; $name $test) };
    }
    #[allow(unused_macros)]
    macro_rules! skiptest {
        ($name:literal $test:block) => { test!(test_skip; $name $test) };
    }

    // game setup tests
    test!("Setup without args should use random initialization" {
        let first = new_driver().send(Command::setup())?;
        let second = new_driver().send(Command::setup())?;

        assert_ne!(first, second);
    });

    test!("Setup with set seed should use random initialization with given seed" {
        let first = new_driver().send(Command::setup())?;
        let seed = first.clone().setup_ok().seed.unwrap();

        let second = new_driver().send(Command::setup().seed(seed))?;

        assert_eq!(first, second);
    });

    test!("Setup with set blocked_cells" {
        let res = new_driver().send(Command::setup().blocked_cells(&[6u8, 3, 0xC]))?;
        let blocked_cells = res.setup_ok().blocked_cells;

        assert_eq!(blocked_cells, vec![3, 6, 0xC]);
    });

    test!("Setup with set blocked_cells to nothing" {
        let res = new_driver().send(Command::setup().blocked_cells(&[]))?;
        let blocked_cells = res.setup_ok().blocked_cells;

        assert_eq!(blocked_cells, vec![]);
    });

    test!("Setup with set hand candidates" {
        const C1P23_4: Card = Card::physical(1, 2, 3, 4);
        const C5M67_8: Card = Card::magical(5, 6, 7, 8);
        const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, 0xC);
        const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, 0);
        let expected = [
            [C5M67_8, CDAEF_0, C9XAB_C, C5M67_8, C1P23_4],
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C1P23_4, C5M67_8, CDAEF_0, C5M67_8, C9XAB_C],
        ];
        let res = new_driver().send(Command::setup().hand_candidates(&expected))?;
        let actual = res.setup_ok().hand_candidates;

        assert_eq!(actual, expected);
    });

    // pre-game tests
    test!("P1 hand selection, ok" {
        let mut driver = new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        let res = driver.send(Command::pick_hand(1))?; // shouldn't error

        assert!(matches!(res, Response::PickHandOk));
    });

    test!("P1 hand selection, invalid number" {
        let mut driver = new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;

        let reason = driver.send(Command::pick_hand(3))?.pick_hand_err();

        assert_eq!(reason, "Invalid Pick '3', expected a number from 0 to 2");
    });

    test!("P2 hand selection, ok" {
        let mut driver = new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let res = driver.send(Command::pick_hand(2))?; // shouldn't error

        assert!(matches!(res, Response::PickHandOk));
    });

    test!("P2 hand selection, invalid number" {
        let mut driver = new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let reason = driver.send(Command::pick_hand(3))?.pick_hand_err();

        assert_eq!(reason, "Invalid Pick '3', expected a number from 0 to 2");
    });

    test!("P2 hand selection, hand already selected" {
        let mut driver = new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let reason = driver.send(Command::pick_hand(0))?.pick_hand_err();

        assert_eq!(reason, "Hand 0 has already been picked");
    });

    // in-game tests
    fn setup_default() -> Command {
        Command::setup()
            .seed(0)
            .battle_system(BattleSystem::OriginalApprox)
            .blocked_cells(&[])
            .hand_candidates(&HAND_CANDIDATES)
    }

    test!("place card with no interaction" {
        let mut driver = new_driver();
        driver.send(setup_default())?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        let events = driver.send(Command::place_card(1, 5))?.place_card_ok();

        assert_eq!(events, vec![]);
    });

    test!("place card that flips one other card" {
        let mut driver = new_driver();
        let defender = Card::physical(0, 0, 0, 0);
        let attacker = Card::physical(0, 0, 0, Arrows::UP.0);
        let hand_candidates = [
            [CARD, CARD, defender, CARD, CARD],
            [CARD, attacker, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;
        driver.send(Command::place_card(2, 1))?;

        let events = driver.send(Command::place_card(1, 5))?.place_card_ok();

        assert_eq!(events, vec![Event::flip(1)]);
    });

    test!("place card that flips multiple other cards" {
        let mut driver = new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 1))?;
        driver.send(Command::place_card(0, 2))?; // att
        driver.send(Command::place_card(1, 6))?;
        driver.send(Command::place_card(1, 0xA))?; // att
        driver.send(Command::place_card(2, 4))?;
        driver.send(Command::place_card(2, 8))?; // att
        driver.send(Command::place_card(3, 0))?;

        let mut events = driver.send(Command::place_card(4, 5))?.place_card_ok();
        events.sort_unstable_by_key(|int| match int {
            Event::Flip { cell } => *cell,
            _ => unreachable!(),
        });

        assert_eq!(
            events,
            vec![
                Event::flip(0),
                Event::flip(1),
                Event::flip(4),
                Event::flip(6)
            ]
        );
    });

    test!("place card that results in a battle, attacker wins" {
        let mut driver = new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0])
                .hand_candidates(&hand_candidates),
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let events = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 0xC, 0xC6),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(0),
            ]
        );
    });

    test!("place card that results in a battle, defender wins" {
        let mut driver = new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[0, 255])
                .hand_candidates(&hand_candidates),
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let events = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 0xC, 0),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0x36),
                    BattleWinner::Defender,
                ),
                Event::flip(1),
            ]
        );
    });

    test!("place card that results in a battle, draw" {
        let mut driver = new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0x3, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[100, 100])
                .hand_candidates(&hand_candidates),
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let events = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 3, 21),
                    Battler::new(0, Digit::PhysicalDefense, 3, 21),
                    BattleWinner::None,
                ),
                Event::flip(1),
            ]
        );
    });

    test!("place card that results in a combo" {
        let mut driver = new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0])
                .hand_candidates(&hand_candidates),
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 5))?; // defender
        driver.send(Command::PlaceCard { card: 1, cell: 0xF })?; // out of the way
        driver.send(Command::place_card(1, 0))?; // will be combo'd

        let events = driver.send(Command::place_card(0, 9))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(9, Digit::Attack, 0xC, 0xC6),
                    Battler::new(5, Digit::PhysicalDefense, 3, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(5),
                Event::combo_flip(0),
            ]
        );
    });

    test!("place card that results in a choice" {
        let mut driver = new_driver();
        let defender1 = Card::physical(0, 3, 7, Arrows::ALL.0);
        let defender2 = Card::physical(0, 9, 4, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender1, defender2, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0, 0, 255])
                .hand_candidates(&hand_candidates),
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender 1
        driver.send(Command::PlaceCard { card: 1, cell: 0xF })?; // out of the way
        driver.send(Command::place_card(1, 8))?; // defender 2

        let choices = driver
            .send(Command::place_card(0, 4))?
            .place_card_pick_battle();

        assert_eq!(choices, vec![0, 8]);

        let events = driver.send(Command::pick_battle(8))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0xC6),
                    Battler::new(8, Digit::MagicalDefense, 4, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(8),
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0x36),
                    BattleWinner::Defender,
                ),
                Event::flip(4),
                Event::combo_flip(8),
            ]
        );
    });

    test!("place card that ends the game in a draw" {
        let mut driver = new_driver();
        driver.send(setup_default())?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;
        driver.send(Command::place_card(0, 1))?;

        driver.send(Command::place_card(1, 2))?;
        driver.send(Command::place_card(1, 3))?;

        driver.send(Command::place_card(2, 4))?;
        driver.send(Command::place_card(2, 5))?;

        driver.send(Command::place_card(3, 6))?;
        driver.send(Command::place_card(3, 7))?;

        driver.send(Command::place_card(4, 8))?;

        let events = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::GameOver { winner: None }]);
    });

    test!("place card that ends the game in player 1 drawing" {
        let mut driver = new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;
        driver.send(Command::place_card(0, 1))?;

        driver.send(Command::place_card(1, 2))?;
        driver.send(Command::place_card(1, 3))?;

        driver.send(Command::place_card(2, 4))?;
        driver.send(Command::place_card(2, 5))?;

        driver.send(Command::place_card(3, 6))?;
        driver.send(Command::place_card(3, 7))?;

        driver.send(Command::place_card(4, 8))?;

        let events = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::game_over(Some(Player::P1))]);
    });

    test!("place card that ends the game in player 2 drawing" {
        let mut driver = new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;
        driver.send(Command::place_card(0, 1))?;

        driver.send(Command::place_card(1, 2))?;
        driver.send(Command::place_card(1, 3))?;

        driver.send(Command::place_card(2, 4))?;
        driver.send(Command::place_card(2, 5))?;

        driver.send(Command::place_card(3, 6))?;
        driver.send(Command::place_card(3, 7))?;

        driver.send(Command::place_card(4, 8))?;

        let events = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::game_over(Some(Player::P2))]);
    });

    harness.run();
}
