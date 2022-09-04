use pretty_assertions::{assert_eq, assert_ne};

use crate::{
    driver::{BattleWinner, Battler, Command, Digit, ErrorResponse, Event, Response},
    harness::{Harness, Suite},
    Arrows, BattleSystem, Card, Player,
};

mod helpers;
use helpers::*;

fn game_setup_tests(s: &mut Suite<Ctx>) {
    test!(s "Setup without args should use random initialization"; |ctx| {
        let first = ctx.new_driver().send(Command::setup())?;
        let second = ctx.new_driver().send(Command::setup())?;

        assert_ne!(first, second);
    });

    test!(s "Setup with set seed should use random initialization with given seed"; |ctx| {
        let first = ctx.new_driver().send(Command::setup())?;
        let seed = first.clone().setup_ok().seed.unwrap();

        let second = ctx.new_driver().send(Command::setup().seed(seed))?;

        assert_eq!(first, second);
    });

    test!(s "Setup with set blocked_cells"; |ctx| {
        let res = ctx.new_driver().send(Command::setup().blocked_cells(&[6u8, 3, 0xC]))?;
        let blocked_cells = res.setup_ok().blocked_cells;

        assert_eq!(blocked_cells, vec![3, 6, 0xC]);
    });

    test!(s "Setup with set blocked_cells to nothing"; |ctx| {
        let res = ctx.new_driver().send(Command::setup().blocked_cells(&[]))?;
        let blocked_cells = res.setup_ok().blocked_cells;

        assert_eq!(blocked_cells, vec![]);
    });

    test!(s "Setup with set hand candidates"; |ctx| {
        const C1P23_4: Card = Card::physical(1, 2, 3, Arrows(4));
        const C5M67_8: Card = Card::magical(5, 6, 7, Arrows(8));
        const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, Arrows(0xC));
        const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, Arrows(0));
        let expected = [
            [C5M67_8, CDAEF_0, C9XAB_C, C5M67_8, C1P23_4],
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C1P23_4, C5M67_8, CDAEF_0, C5M67_8, C9XAB_C],
        ];
        let res = ctx.new_driver().send(Command::setup().hand_candidates(&expected))?;
        let actual = res.setup_ok().hand_candidates;

        assert_eq!(actual, expected);
    });
}

fn pre_game_tests(s: &mut Suite<Ctx>) {
    test!(s "P1 hand selection, ok"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?.setup_ok();
        let res = driver.send(Command::pick_hand(1))?; // shouldn't error

        assert!(matches!(res, Response::PickHandOk));
    });

    test!(s "P1 hand selection, invalid number"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?.setup_ok();

        let error = driver.send(Command::pick_hand(3))?.error();

        assert_eq!(error, ErrorResponse::InvalidHandPick { hand: 3 });
    });

    test!(s "P2 hand selection, ok"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();

        let res = driver.send(Command::pick_hand(2))?; // shouldn't error

        assert!(matches!(res, Response::PickHandOk));
    });

    test!(s "P2 hand selection, invalid number"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();

        let error = driver.send(Command::pick_hand(7))?.error();

        assert_eq!(error, ErrorResponse::InvalidHandPick { hand: 7 });
    });

    test!(s "P2 hand selection, hand already selected"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();

        let error = driver.send(Command::pick_hand(0))?.error();

        assert_eq!(error, ErrorResponse::HandAlreadyPicked { hand: 0 });
    });
}

fn in_game_tests(s: &mut Suite<Ctx>) {
    fn setup_default() -> Command {
        Command::setup()
            .seed(0)
            .battle_system(BattleSystem::Test)
            .blocked_cells(&[])
            .hand_candidates(&HAND_CANDIDATES)
    }

    test!(s "place card with no interaction"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default())?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        let (_, events) = driver.send(Command::place_card(1, 5))?.place_card_ok();

        assert_eq!(events, vec![Event::turn_p2()]);
    });

    test!(s "error if the card has already been played"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default())?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(1, 5))?.place_card_ok();
        driver.send(Command::place_card(0, 0))?.place_card_ok();

        let error = driver.send(Command::place_card(1, 3))?.error();

        assert_eq!(error, ErrorResponse::CardAlreadyPlayed { card: 1 });
    });

    test!(s "error if the cell played on is blocked"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default().blocked_cells(&[0xB]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        let error = driver.send(Command::place_card(0, 0xB))?.error();

        assert_eq!(error, ErrorResponse::CellIsNotEmpty { cell: 0xB });
    });

    test!(s "error if the cell played on already has a card placed"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default().blocked_cells(&[0xB]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 3))?.place_card_ok();
        let error = driver.send(Command::place_card(0, 3))?.error();

        assert_eq!(error, ErrorResponse::CellIsNotEmpty { cell: 3 });
    });

    test!(s "place card that flips one other card"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::RIGHT);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, attacker, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // should flip, is pointed to and belongs to p1
        driver.send(Command::place_card(0, 5))?.place_card_ok(); // shouldn't flip, belongs to p2
        driver.send(Command::place_card(1, 8))?.place_card_ok(); // shouldn't flip, not pointed to

        let (_, events) = driver.send(Command::place_card(1, 4))?.place_card_ok();

        assert_eq!(events, vec![Event::flip(0), Event::turn_p1()]);
    });

    test!(s "place card that flips multiple other cards"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 1))?.place_card_ok();
        driver.send(Command::place_card(0, 2))?.place_card_ok(); // att
        driver.send(Command::place_card(1, 6))?.place_card_ok();
        driver.send(Command::place_card(1, 0xA))?.place_card_ok(); // att
        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 8))?.place_card_ok(); // att
        driver.send(Command::place_card(3, 0))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(4, 5))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::flip(0),
                Event::flip(1),
                Event::flip(4),
                Event::flip(6),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "flips events should be ordered by increasing cell number"; |ctx| {
        let mut driver = ctx.new_driver();
        let flipper = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = {
            let c = |arrows| CARD.arrows(arrows);
            [
                [CARD, c(Arrows::LEFT), c(Arrows::UP), c(Arrows::RIGHT), c(Arrows::RIGHT)],
                [CARD, CARD, CARD, CARD, flipper],
                [CARD, CARD, CARD, CARD, CARD],
            ]
        };
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0x1))?.place_card_ok();
        driver.send(Command::place_card(0, 0x2))?.place_card_ok();
        driver.send(Command::place_card(1, 0x3))?.place_card_ok(); // flip card on 1
        driver.send(Command::place_card(1, 0x7))?.place_card_ok();
        driver.send(Command::place_card(2, 0xB))?.place_card_ok(); // flip card on 6
        driver.send(Command::place_card(2, 0xA))?.place_card_ok();
        driver.send(Command::place_card(3, 0x9))?.place_card_ok(); // flip card on 9
        driver.send(Command::place_card(3, 0x5))?.place_card_ok();
        driver.send(Command::place_card(4, 0x4))?.place_card_ok(); // flip card on 5

        // all cards on board now belong to P1

        // flip 8 surrounding cards
        let (_, events) = driver.send(Command::place_card(4, 6))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::flip(0x1),
                Event::flip(0x2),
                Event::flip(0x3),
                Event::flip(0x5),
                Event::flip(0x7),
                Event::flip(0x9),
                Event::flip(0xA),
                Event::flip(0xB),
                Event::game_over(Some(Player::P2)),
            ]
        );
    });

    test!(s "place card that results in a battle, attacker wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 0xC, 0xCF),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(0),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "place card that results in a battle, defender wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[0, 255])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 0xC, 0),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0x3F),
                    BattleWinner::Defender,
                ),
                Event::flip(1),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "place card that results in a battle, draw"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL);
        let attacker = Card::exploit(0x3, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[100, 100])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(0, 1))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(1, Digit::Attack, 3, 25),
                    Battler::new(0, Digit::PhysicalDefense, 3, 25),
                    BattleWinner::None,
                ),
                Event::flip(1),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "flip other undefended cards after attacker wins battle"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::DOWN);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender
        driver.send(Command::place_card(1, 3))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(1, 1))?.place_card_ok();
        driver.send(Command::place_card(2, 7))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(2, 5))?.place_card_ok();
        driver.send(Command::place_card(3, 0xB))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(3, 9))?.place_card_ok();
        driver.send(Command::place_card(4, 0xF))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(4, 8))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(0, 4))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0xCF),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(0),
                Event::flip(1),
                Event::flip(5),
                Event::flip(8),
                Event::flip(9),
                Event::game_over(Some(Player::P2)),
            ]
        );
    });

    test!(s "don't flip other undefended cards after attacker loses battle"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::DOWN);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[0, 255])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender
        driver.send(Command::place_card(1, 3))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(1, 1))?.place_card_ok();
        driver.send(Command::place_card(2, 7))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(2, 5))?.place_card_ok();
        driver.send(Command::place_card(3, 0xB))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(3, 9))?.place_card_ok();
        driver.send(Command::place_card(4, 0xF))?.place_card_ok(); // out of the way

        driver.send(Command::place_card(4, 8))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(0, 4))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0x3F),
                    BattleWinner::Defender,
                ),
                Event::flip(4),
                Event::game_over(Some(Player::P1)),
            ]
        );
    });

    test!(s "place card that results in a combo"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 3, 7, Arrows::ALL);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 5))?.place_card_ok(); // defender
        driver.send(Command::place_card(1, 0xF))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(1, 0))?.place_card_ok(); // will be combo'd

        let (_, events) = driver.send(Command::place_card(0, 9))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(9, Digit::Attack, 0xC, 0xCF),
                    Battler::new(5, Digit::PhysicalDefense, 3, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(5),
                Event::combo_flip(0),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "place card that results in a choice"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender1 = Card::physical(0, 3, 7, Arrows::ALL);
        let defender2 = Card::physical(0, 9, 4, Arrows::ALL);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender1, defender2, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0, 0, 255])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender 1
        driver.send(Command::place_card(1, 0xF))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(1, 8))?.place_card_ok(); // defender 2

        let (choices, _) = driver.send(Command::place_card(0, 4))?.place_card_ok();

        assert_eq!(choices, vec![0, 8]);

        let (_, events) = driver.send(Command::pick_battle(8))?.place_card_ok();

        assert_eq!(
            events,
            vec![
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0xCF),
                    Battler::new(8, Digit::MagicalDefense, 4, 0),
                    BattleWinner::Attacker,
                ),
                Event::flip(8),
                Event::battle(
                    Battler::new(4, Digit::Attack, 0xC, 0),
                    Battler::new(0, Digit::PhysicalDefense, 3, 0x3F),
                    BattleWinner::Defender,
                ),
                Event::flip(4),
                Event::combo_flip(8),
                Event::turn_p1(),
            ]
        );
    });

    test!(s "error if battle choice isn't valid"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender1 = Card::physical(0, 0, 0, Arrows::DOWN);
        let defender2 = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::DOWN);
        let hand_candidates = [
            [defender1, defender2, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender 1
        driver.send(Command::place_card(1, 3))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(1, 8))?.place_card_ok(); // defender 2
        driver.send(Command::place_card(0, 4))?.place_card_ok();

        let error = driver.send(Command::pick_battle(0xC))?.error();

        assert_eq!(error, ErrorResponse::InvalidBattlePick { cell: 0xC });
    });

    test!(s "continue offering choices when multiple battles are still available"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender0 = Card::physical(0, 2, 0, Arrows::DOWN);
        let defender1 = Card::physical(0, 4, 0, Arrows::LEFT);
        let defender2 = Card::physical(0, 6, 0, Arrows::UP);
        let defender3 = Card::physical(0, 8, 0, Arrows::UP_LEFT);
        let attacker = Card::physical(1, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender0, defender1, defender2, defender3, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(
            setup_default()
                .rolls(&[255, 0, 255, 0, 255, 0, 255, 0])
                .hand_candidates(&hand_candidates),
        )?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender 0
        driver.send(Command::place_card(1, 3))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(1, 5))?.place_card_ok(); // defender 1
        driver.send(Command::place_card(2, 7))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(2, 8))?.place_card_ok(); // defender 2
        driver.send(Command::place_card(3, 0xB))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(3, 9))?.place_card_ok(); // defender 2

        let (choices, events) = driver.send(Command::place_card(0, 4))?.place_card_ok();
        assert_eq!(choices, vec![0, 5, 8, 9]);
        assert_eq!(events, vec![]);

        let (choices, events) = driver.send(Command::pick_battle(5))?.place_card_ok();
        assert_eq!(choices, vec![0, 8, 9]);
        assert_eq!(events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(5, Digit::PhysicalDefense, 4, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(5),
        ]);

        let (choices, events) = driver.send(Command::pick_battle(8))?.place_card_ok();
        assert_eq!(choices, vec![0, 9]);
        assert_eq!(events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(8, Digit::PhysicalDefense, 6, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(8),
        ]);

        let (choices, events) = driver.send(Command::pick_battle(0))?.place_card_ok();
        assert_eq!(choices, vec![]);
        assert_eq!(events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(0, Digit::PhysicalDefense, 2, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(0),
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(9, Digit::PhysicalDefense, 8, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(9),
            Event::turn_p1(),
        ]);
    });

    test!(s "don't continue offering choices if attacker loses"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender0 = Card::physical(0, 2, 0, Arrows::DOWN);
        let defender1 = Card::physical(0, 4, 0, Arrows::LEFT);
        let defender2 = Card::physical(0, 6, 0, Arrows::UP);
        let defender3 = Card::physical(0, 8, 0, Arrows::UP_LEFT);
        let attacker = Card::physical(1, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [defender0, defender1, defender2, defender3, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().rolls(&[0, 255]).hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender 0
        driver.send(Command::place_card(1, 3))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(1, 5))?.place_card_ok(); // defender 1
        driver.send(Command::place_card(2, 7))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(2, 8))?.place_card_ok(); // defender 2
        driver.send(Command::place_card(3, 0xB))?.place_card_ok(); // out of the way
        driver.send(Command::place_card(3, 9))?.place_card_ok(); // defender 2

        let (choices, events) = driver.send(Command::place_card(0, 4))?.place_card_ok();
        assert_eq!(choices, vec![0, 5, 8, 9]);
        assert_eq!(events, vec![]);

        let (choices, events) = driver.send(Command::pick_battle(5))?.place_card_ok();
        assert_eq!(choices, vec![]);
        assert_eq!(events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0),
                Battler::new(5, Digit::PhysicalDefense, 4, 0x4F),
                BattleWinner::Defender,
            ),
            Event::flip(4),
            Event::turn_p1(),
        ]);
    });

    test!(s "place card that ends the game in a draw"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default())?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();

        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();

        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();

        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::GameOver { winner: None }]);
    });

    test!(s "place card that ends the game in player 1 drawing"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();

        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();

        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();

        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::game_over(Some(Player::P1))]);
    });

    test!(s "place card that ends the game in player 2 drawing"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();

        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();

        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();

        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();

        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![Event::game_over(Some(Player::P2))]);
    });

    test!(s "handle game over when attacker loses battle after a choice"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender0 = Card::physical(0, 0, 0, Arrows::DOWN);
        let defender1 = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::DOWN);
        let hand_candidates = [
            [defender0, defender1, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().rolls(&[0, 255]).hand_candidates(&hand_candidates))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok(); // defender0
        driver.send(Command::place_card(0, 3))?.place_card_ok();

        driver.send(Command::place_card(1, 8))?.place_card_ok(); // defender1
        driver.send(Command::place_card(1, 7))?.place_card_ok();

        driver.send(Command::place_card(2, 0xB))?.place_card_ok();
        driver.send(Command::place_card(2, 0xF))?.place_card_ok();

        driver.send(Command::place_card(3, 2))?.place_card_ok();
        driver.send(Command::place_card(3, 6))?.place_card_ok();

        driver.send(Command::place_card(4, 0xA))?.place_card_ok();

        driver.send(Command::place_card(4, 4))?.place_card_ok();
        let (_, events) = driver.send(Command::pick_battle(0))?.place_card_ok();

        assert_eq!(events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 0, 0),
                Battler::new(0, Digit::PhysicalDefense, 0, 0xF),
                BattleWinner::Defender,
            ),
            Event::flip(4),
            Event::game_over(Some(Player::P1)),
        ]);
    });

    test!(s "combo flip cards that are pointed to by the defender if they lose"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 0, 0, Arrows::ALL);
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, defender, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[255, 0]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 1))?.place_card_ok(); // will flip
        driver.send(Command::place_card(0, 3))?.place_card_ok();

        driver.send(Command::place_card(1, 4))?.place_card_ok(); // will flip
        driver.send(Command::place_card(1, 7))?.place_card_ok();

        driver.send(Command::place_card(2, 6))?.place_card_ok(); // will flip
        driver.send(Command::place_card(2, 0xB))?.place_card_ok();

        driver.send(Command::place_card(3, 5))?.place_card_ok(); // defender

        let (_, events) = driver.send(Command::place_card(3, 9))?.place_card_ok();

        assert_eq!(events, vec![
            Event::battle(
                Battler::new(9, Digit::Attack, 0, 0xF),
                Battler::new(5, Digit::PhysicalDefense, 0, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(5),
            Event::combo_flip(1),
            Event::combo_flip(4),
            Event::combo_flip(6),
            Event::turn_p1(),
        ]);
    });

    test!(s "combo flip cards that are pointed to by the attacker if they lose"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, defender, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[0, 255]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 3))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok(); // will flip

        driver.send(Command::place_card(1, 7))?.place_card_ok();
        driver.send(Command::place_card(1, 4))?.place_card_ok(); // will flip

        driver.send(Command::place_card(2, 0xB))?.place_card_ok();
        driver.send(Command::place_card(2, 6))?.place_card_ok(); // will flip

        driver.send(Command::place_card(3, 9))?.place_card_ok(); // defender

        let (_, events) = driver.send(Command::place_card(3, 5))?.place_card_ok();

        assert_eq!(events, vec![
            Event::battle(
                Battler::new(5, Digit::Attack, 0, 0),
                Battler::new(9, Digit::PhysicalDefense, 0, 0xF),
                BattleWinner::Defender,
            ),
            Event::flip(5),
            Event::combo_flip(1),
            Event::combo_flip(4),
            Event::combo_flip(6),
            Event::turn_p1(),
        ]);
    });

    test!(s "game should be over once all cards have been played, player 1 wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD, CARD.arrows(Arrows::ALL), CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[0, 255]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();
        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();
        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();
        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();
        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![
            Event::game_over(Some(Player::P1)),
        ]);
    });

    test!(s "game should be over once all cards have been played, player 2 wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD.arrows(Arrows::ALL), CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[0, 255]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();
        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();
        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();
        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();
        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![
            Event::game_over(Some(Player::P2)),
        ]);
    });

    test!(s "game should be over once all cards have been played, it's a draw"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[0, 255]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 0))?.place_card_ok();
        driver.send(Command::place_card(0, 1))?.place_card_ok();
        driver.send(Command::place_card(1, 2))?.place_card_ok();
        driver.send(Command::place_card(1, 3))?.place_card_ok();
        driver.send(Command::place_card(2, 4))?.place_card_ok();
        driver.send(Command::place_card(2, 5))?.place_card_ok();
        driver.send(Command::place_card(3, 6))?.place_card_ok();
        driver.send(Command::place_card(3, 7))?.place_card_ok();

        driver.send(Command::place_card(4, 8))?.place_card_ok();
        let (_, events) = driver.send(Command::place_card(4, 9))?.place_card_ok();

        assert_eq!(events, vec![
            Event::game_over(None),
        ]);
    });

    test!(s "don't flip back undefended cards if they are flipped due to combos"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD.arrows(Arrows::ALL), CARD, CARD, CARD],
            [CARD.arrows(Arrows::ALL), CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[255, 0]))?.setup_ok();
        driver.send(Command::pick_hand(0))?.pick_hand_ok();
        driver.send(Command::pick_hand(1))?.pick_hand_ok();

        driver.send(Command::place_card(0, 4))?.place_card_ok();
        driver.send(Command::place_card(0, 0))?.place_card_ok(); // flips the card on 4

        // placed card points to both other cards, attacker wins,
        // flips card on 0 and card on 4 get's combo flipped
        let (_, events) = driver.send(Command::place_card(1, 5))?.place_card_ok();

        assert_eq!(events, vec![
            Event::battle(
                Battler::new(5, Digit::Attack, 0, 0xF),
                Battler::new(0, Digit::PhysicalDefense, 0, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(0),
            Event::combo_flip(4),
            Event::turn_p2(),
        ]);
    });
}

pub(crate) fn run(implementation: String) {
    let mut harness = Harness::new(Ctx { implementation });

    game_setup_tests(suite!(harness "Game Setup"));

    pre_game_tests(suite!(harness "Pre-Game"));

    in_game_tests(suite!(harness "In-Game"));

    harness.run();
}
