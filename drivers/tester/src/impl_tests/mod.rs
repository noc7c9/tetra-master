// board cells reference
//
//  0 | 1 | 2 | 3
// ---+---+---+---
//  4 | 5 | 6 | 7
// ---+---+---+---
//  8 | 9 | A | B
// ---+---+---+---
//  C | D | E | F

use pretty_assertions::{assert_eq, assert_ne};
use tetra_master_core::{
    command, response, Arrows, BattleSystem, BattleWinner, Battler, Card, Digit, Driver,
    ErrorResponse, Event, Player,
};

use crate::harness::{Harness, Suite};

mod helpers;
use helpers::*;

pub(crate) enum Ctx<'a> {
    External { implementation: &'a str },
    Reference,
}

impl<'a> Ctx<'a> {
    pub(super) fn new_driver(&self) -> Driver {
        match self {
            Ctx::External { implementation } => Driver::external(implementation),
            Ctx::Reference => Driver::reference(),
        }
    }
}

fn setup_default() -> command::Setup {
    Command::setup()
        .seed(0)
        .battle_system(BattleSystem::Test)
        .blocked_cells(&[])
        .hand_candidates(&HAND_CANDIDATES)
}

fn game_setup_tests(s: &mut Suite<Ctx>) {
    test!(s "Setup without args should use random initialization"; |ctx| {
        let first = ctx.new_driver().send(Command::setup())?;
        let second = ctx.new_driver().send(Command::setup())?;

        assert_ne!(first, second);
    });

    test!(s "Setup with set seed should use random initialization with given seed"; |ctx| {
        let first = ctx.new_driver().send(Command::setup())?;
        let seed = first.seed.unwrap();

        let second = ctx.new_driver().send(Command::setup().seed(seed))?;

        assert_eq!(first, second);
    });

    test!(s "Setup with set blocked_cells"; |ctx| {
        let res = ctx.new_driver().send(Command::setup().blocked_cells(&[6u8, 3, 0xC]))?;
        let blocked_cells = res.blocked_cells;

        assert_eq!(blocked_cells, vec![3, 6, 0xC]);
    });

    test!(s "Setup with set blocked_cells to nothing"; |ctx| {
        let res = ctx.new_driver().send(Command::setup().blocked_cells(&[]))?;
        let blocked_cells = res.blocked_cells;

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
        let actual = res.hand_candidates;

        assert_eq!(actual, expected);
    });
}

fn pre_game_tests(s: &mut Suite<Ctx>) {
    test!(s "P1 hand selection, ok"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        let res = driver.send(Command::pick_hand(1))?; // shouldn't error

        assert!(matches!(res, response::PickHandOk));
    });

    test!(s "P1 hand selection, invalid number"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;

        let error = driver.send(Command::pick_hand(3)).error();

        assert_eq!(error, ErrorResponse::InvalidHandPick { hand: 3 });
    });

    test!(s "P2 hand selection, ok"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let res = driver.send(Command::pick_hand(2))?; // shouldn't error

        assert!(matches!(res, response::PickHandOk));
    });

    test!(s "P2 hand selection, invalid number"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let error = driver.send(Command::pick_hand(7)).error();

        assert_eq!(error, ErrorResponse::InvalidHandPick { hand: 7 });
    });

    test!(s "P2 hand selection, hand already selected"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(Command::setup().hand_candidates(&HAND_CANDIDATES))?;
        driver.send(Command::pick_hand(0))?;

        let error = driver.send(Command::pick_hand(0)).error();

        assert_eq!(error, ErrorResponse::HandAlreadyPicked { hand: 0 });
    });
}

fn in_game_tests(s: &mut Suite<Ctx>) {
    test!(s "place card with no interaction"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default())?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        let events = driver.send(Command::place_card(1, 5))?.events;

        assert_eq!(events, vec![Event::turn_p2()]);
    });
    test!(s "error if the card has already been played"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default())?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(1, 5))?;
        driver.send(Command::place_card(0, 0))?;

        let error = driver.send(Command::place_card(1, 3)).error();

        assert_eq!(error, ErrorResponse::CardAlreadyPlayed { card: 1 });
    });
    test!(s "error if the cell played on is blocked"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default().blocked_cells(&[0xB]))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        let error = driver.send(Command::place_card(0, 0xB)).error();

        assert_eq!(error, ErrorResponse::CellIsNotEmpty { cell: 0xB });
    });
    test!(s "error if the cell played on already has a card placed"; |ctx| {
        let mut driver = ctx.new_driver();
        driver.send(setup_default().blocked_cells(&[0xB]))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 3))?;
        let error = driver.send(Command::place_card(0, 3)).error();

        assert_eq!(error, ErrorResponse::CellIsNotEmpty { cell: 3 });
    });

    let ss = suite!(s "Flips");
    test!(ss "place card that flips one other card"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::RIGHT);
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, attacker, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // should flip
        driver.send(Command::place_card(0, 5))?; // shouldn't flip, belongs to p2
        driver.send(Command::place_card(1, 8))?; // shouldn't flip, not pointed to

        let events = driver.send(Command::place_card(1, 4))?.events;

        assert_eq!(events, vec![Event::flip(0), Event::turn_p1()]);
    });
    test!(ss "place card that flips multiple other cards"; |ctx| {
        let mut driver = ctx.new_driver();
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
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

        let events = driver.send(Command::place_card(4, 5))?.events;

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
    test!(ss "flips events should be ordered by increasing cell number"; |ctx| {
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
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0x1))?;
        driver.send(Command::place_card(0, 0x2))?;
        driver.send(Command::place_card(1, 0x3))?; // flip card on 1
        driver.send(Command::place_card(1, 0x7))?;
        driver.send(Command::place_card(2, 0xB))?; // flip card on 6
        driver.send(Command::place_card(2, 0xA))?;
        driver.send(Command::place_card(3, 0x9))?; // flip card on 9
        driver.send(Command::place_card(3, 0x5))?;
        driver.send(Command::place_card(4, 0x4))?; // flip card on 5

        // all cards on board now belong to P1

        // flip 8 surrounding cards
        let  events = driver.send(Command::place_card(4, 6))?.events;

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

    let ss = suite!(s "Battles");
    test!(ss "place card that results in a battle, attacker wins"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let  events = driver.send(Command::place_card(0, 1))?.events;

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
    test!(ss "place card that results in a battle, defender wins"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let  events = driver.send(Command::place_card(0, 1))?.events;

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
    test!(ss "place card that results in a battle, draw"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;

        let  events = driver.send(Command::place_card(0, 1))?.events;

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
    test!(ss "flip other undefended cards after attacker wins battle"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender
        driver.send(Command::place_card(1, 3))?; // out of the way

        driver.send(Command::place_card(1, 1))?;
        driver.send(Command::place_card(2, 7))?; // out of the way

        driver.send(Command::place_card(2, 5))?;
        driver.send(Command::place_card(3, 0xB))?; // out of the way

        driver.send(Command::place_card(3, 9))?;
        driver.send(Command::place_card(4, 0xF))?; // out of the way

        driver.send(Command::place_card(4, 8))?;

        let events = driver.send(Command::place_card(0, 4))?.events;

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
    test!(ss "don't flip other undefended cards after attacker loses battle"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender
        driver.send(Command::place_card(1, 3))?; // out of the way

        driver.send(Command::place_card(1, 1))?;
        driver.send(Command::place_card(2, 7))?; // out of the way

        driver.send(Command::place_card(2, 5))?;
        driver.send(Command::place_card(3, 0xB))?; // out of the way

        driver.send(Command::place_card(3, 9))?;
        driver.send(Command::place_card(4, 0xF))?; // out of the way

        driver.send(Command::place_card(4, 8))?;

        let events = driver.send(Command::place_card(0, 4))?.events;

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

    let ss = suite!(s "Combos");
    test!(ss "place card that results in a combo"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 5))?; // defender
        driver.send(Command::place_card(1, 0xF))?; // out of the way
        driver.send(Command::place_card(1, 0))?; // will be combo'd

        let events = driver.send(Command::place_card(0, 9))?.events;

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
    test!(ss "combo flip cards that are pointed to by the defender if they lose"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 0, 0, Arrows::ALL);
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, defender, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[255, 0]))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 1))?; // will flip
        driver.send(Command::place_card(0, 3))?;

        driver.send(Command::place_card(1, 4))?; // will flip
        driver.send(Command::place_card(1, 7))?;

        driver.send(Command::place_card(2, 6))?; // will flip
        driver.send(Command::place_card(2, 0xB))?;

        driver.send(Command::place_card(3, 5))?; // defender

        let  events = driver.send(Command::place_card(3, 9))?.events;

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
    test!(ss "combo flip cards that are pointed to by the attacker if they lose"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::ALL);
        let hand_candidates = [
            [CARD, CARD, CARD, defender, CARD],
            [CARD, CARD, CARD, attacker, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[0, 255]))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 3))?;
        driver.send(Command::place_card(0, 1))?; // will flip

        driver.send(Command::place_card(1, 7))?;
        driver.send(Command::place_card(1, 4))?; // will flip

        driver.send(Command::place_card(2, 0xB))?;
        driver.send(Command::place_card(2, 6))?; // will flip

        driver.send(Command::place_card(3, 9))?; // defender

        let events = driver.send(Command::place_card(3, 5))?.events;

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
    test!(ss "don't flip back undefended cards if they are flipped due to combos"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD.arrows(Arrows::ALL), CARD, CARD, CARD],
            [CARD.arrows(Arrows::ALL), CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates).rolls(&[255, 0]))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 4))?;
        driver.send(Command::place_card(0, 0))?; // flips the card on 4

        // placed card points to both other cards, attacker wins,
        // flips card on 0 and card on 4 get's combo flipped
        let events = driver.send(Command::place_card(1, 5))?.events;

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

    let ss = suite!(s "Battle Choices");
    test!(ss "place card that results in a choice"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender 1
        driver.send(Command::place_card(1, 0xF))?; // out of the way
        driver.send(Command::place_card(1, 8))?; // defender 2

        let choices = driver.send(Command::place_card(0, 4))?.pick_battle;

        assert_eq!(choices, vec![0, 8]);

        let events = driver.send(Command::pick_battle(8))?.events;

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
    test!(ss "error if battle choice isn't valid"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender1 = Card::physical(0, 0, 0, Arrows::DOWN);
        let defender2 = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::DOWN);
        let hand_candidates = [
            [defender1, defender2, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender 1
        driver.send(Command::place_card(1, 3))?; // out of the way
        driver.send(Command::place_card(1, 8))?; // defender 2
        driver.send(Command::place_card(0, 4))?;

        let error = driver.send(Command::pick_battle(0xC)).error();

        assert_eq!(error, ErrorResponse::InvalidBattlePick { cell: 0xC });
    });
    test!(ss "continue offering choices when multiple battles are still available"; |ctx| {
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
        )?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender 0
        driver.send(Command::place_card(1, 3))?; // out of the way
        driver.send(Command::place_card(1, 5))?; // defender 1
        driver.send(Command::place_card(2, 7))?; // out of the way
        driver.send(Command::place_card(2, 8))?; // defender 2
        driver.send(Command::place_card(3, 0xB))?; // out of the way
        driver.send(Command::place_card(3, 9))?; // defender 2

        let res = driver.send(Command::place_card(0, 4))?;
        assert_eq!(res.pick_battle, vec![0, 5, 8, 9]);
        assert_eq!(res.events, vec![]);

        let res = driver.send(Command::pick_battle(5))?;
        assert_eq!(res.pick_battle, vec![0, 8, 9]);
        assert_eq!(res.events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(5, Digit::PhysicalDefense, 4, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(5),
        ]);

        let res = driver.send(Command::pick_battle(8))?;
        assert_eq!(res.pick_battle, vec![0, 9]);
        assert_eq!(res.events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0x1F),
                Battler::new(8, Digit::PhysicalDefense, 6, 0),
                BattleWinner::Attacker,
            ),
            Event::flip(8),
        ]);

        let res = driver.send(Command::pick_battle(0))?;
        assert_eq!(res.pick_battle, vec![]);
        assert_eq!(res.events, vec![
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
    test!(ss "don't continue offering choices if attacker loses"; |ctx| {
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
        driver.send(setup_default().rolls(&[0, 255]).hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender 0
        driver.send(Command::place_card(1, 3))?; // out of the way
        driver.send(Command::place_card(1, 5))?; // defender 1
        driver.send(Command::place_card(2, 7))?; // out of the way
        driver.send(Command::place_card(2, 8))?; // defender 2
        driver.send(Command::place_card(3, 0xB))?; // out of the way
        driver.send(Command::place_card(3, 9))?; // defender 2

        let res = driver.send(Command::place_card(0, 4))?;
        assert_eq!(res.pick_battle, vec![0, 5, 8, 9]);
        assert_eq!(res.events, vec![]);

        let res = driver.send(Command::pick_battle(5))?;
        assert_eq!(res.pick_battle, vec![]);
        assert_eq!(res.events, vec![
            Event::battle(
                Battler::new(4, Digit::Attack, 1, 0),
                Battler::new(5, Digit::PhysicalDefense, 4, 0x4F),
                BattleWinner::Defender,
            ),
            Event::flip(4),
            Event::turn_p1(),
        ]);
    });
    test!(ss "handle game over when attacker loses battle after a choice"; |ctx| {
        let mut driver = ctx.new_driver();
        let defender0 = Card::physical(0, 0, 0, Arrows::DOWN);
        let defender1 = Card::physical(0, 0, 0, Arrows::UP);
        let attacker = Card::physical(0, 0, 0, Arrows::UP | Arrows::DOWN);
        let hand_candidates = [
            [defender0, defender1, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, attacker],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().rolls(&[0, 255]).hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?; // defender0
        driver.send(Command::place_card(0, 3))?;

        driver.send(Command::place_card(1, 8))?; // defender1
        driver.send(Command::place_card(1, 7))?;

        driver.send(Command::place_card(2, 0xB))?;
        driver.send(Command::place_card(2, 0xF))?;

        driver.send(Command::place_card(3, 2))?;
        driver.send(Command::place_card(3, 6))?;

        driver.send(Command::place_card(4, 0xA))?;

        driver.send(Command::place_card(4, 4))?;
        let events = driver.send(Command::pick_battle(0))?.events;

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

    let ss = suite!(s "Game Ending");
    test!(ss "game should be over once all cards have been played, player 1 wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD, CARD.arrows(Arrows::ALL), CARD, CARD],
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
        let events = driver.send(Command::place_card(4, 9))?.events;

        assert_eq!(events, vec![
            Event::game_over(Some(Player::P1)),
        ]);
    });
    test!(ss "game should be over once all cards have been played, player 2 wins"; |ctx| {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [CARD, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD.arrows(Arrows::ALL), CARD, CARD],
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
        let events = driver.send(Command::place_card(4, 9))?.events;

        assert_eq!(events, vec![
            Event::game_over(Some(Player::P2)),
        ]);
    });
    test!(ss "game should be over once all cards have been played, it's a draw"; |ctx| {
        let mut driver = ctx.new_driver();
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
        let events = driver.send(Command::place_card(4, 9))?.events;

        assert_eq!(events, vec![
            Event::game_over(None),
        ]);
    });

    stat_selection(suite!(s "Stat Selection"));
}

fn stat_selection(s: &mut Suite<Ctx>) {
    fn run_battle(ctx: &Ctx, attacker: Card, defender: Card) -> anyhow::Result<(Battler, Battler)> {
        let mut driver = ctx.new_driver();
        let hand_candidates = [
            [defender, CARD, CARD, CARD, CARD],
            [attacker, CARD, CARD, CARD, CARD],
            [CARD, CARD, CARD, CARD, CARD],
        ];
        driver.send(setup_default().hand_candidates(&hand_candidates))?;
        driver.send(Command::pick_hand(0))?;
        driver.send(Command::pick_hand(1))?;

        driver.send(Command::place_card(0, 0))?;
        let events = driver.send(Command::place_card(0, 1))?.events;
        let (attacker, defender, _) = events[0].as_battle();
        Ok((attacker, defender))
    }

    const ALL: Arrows = Arrows::ALL;

    // default cards
    const DEFENDER: Card = Card::physical(0x1, 0x2, 0x3, ALL);

    let ss = suite!(s "Attack Stat");

    test!(ss "physical type attacker, picks attack stat"; |ctx| {
        let (stat, _) = run_battle(ctx, Card::physical(0xA, 0xB, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xA);
    });

    test!(ss "magical type attacker, picks attack stat"; |ctx| {
        let (stat, _) = run_battle(ctx, Card::magical(0xA, 0xB, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xA);
    });

    test!(ss "exploit type attacker, picks attack stat"; |ctx| {
        let (stat, _) = run_battle(ctx, Card::exploit(0xA, 0xB, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xA);
    });

    test!(ss "assault type attacker, picks highest stat"; |ctx| {
        // attack stat is highest
        let (stat, _) = run_battle(ctx, Card::assault(0xF, 0xB, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xF);

        // physical defense stat is highest
        let (stat, _) = run_battle(ctx, Card::assault(0xA, 0xE, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::PhysicalDefense);
        assert_eq!(stat.value, 0xE);

        // magical defense stat is highest
        let (stat, _) = run_battle(ctx, Card::assault(0xA, 0xB, 0xD, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::MagicalDefense);
        assert_eq!(stat.value, 0xD);

        // when there is a tie between the attack stat and a defense stat, prefer the attack
        // tie between attack and physical defense
        let (stat, _) = run_battle(ctx, Card::assault(0xE, 0xE, 0xC, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xE);

        // tie between attack and magical defense
        let (stat, _) = run_battle(ctx, Card::assault(0xE, 0xB, 0xE, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xE);

        // tie between all 3 stats
        let (stat, _) = run_battle(ctx, Card::assault(0xE, 0xE, 0xE, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xE);

        // when there is a tie between the defense stats, prefer the physical_defense
        let (stat, _) = run_battle(ctx, Card::assault(0xA, 0xE, 0xE, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::PhysicalDefense);
        assert_eq!(stat.value, 0xE);
    });

    let ss = suite!(s "Defense Stat");

    test!(ss "physical type attacker, picks physical defense stat"; |ctx| {
        let (_, stat) = run_battle(ctx, Card::physical(0, 0, 0, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::PhysicalDefense);
        assert_eq!(stat.value, 0x2);
    });

    test!(ss "magical type attacker, picks physical defense stat"; |ctx| {
        let (_, stat) = run_battle(ctx, Card::magical(0, 0, 0, ALL), DEFENDER)?;
        assert_eq!(stat.digit, Digit::MagicalDefense);
        assert_eq!(stat.value, 0x3);
    });

    test!(ss "exploit type attacker, picks lowest defense stat"; |ctx| {
        // physical defense is lowest
        let defender: Card = Card::physical(0, 0xA, 0xB, ALL);
        let (_, stat) = run_battle(ctx, Card::exploit(0, 0, 0, ALL), defender)?;
        assert_eq!(stat.digit, Digit::PhysicalDefense);
        assert_eq!(stat.value, 0xA);

        // magical defense is lowest
        let defender: Card = Card::physical(0, 0xB, 0xA, ALL);
        let (_, stat) = run_battle(ctx, Card::exploit(0, 0, 0, ALL), defender)?;
        assert_eq!(stat.digit, Digit::MagicalDefense);
        assert_eq!(stat.value, 0xA);
    });

    test!(ss "assault type attacker, picks lowest stat"; |ctx| {
        // physical defense is lowest
        let defender: Card = Card::physical(0xB, 0xA, 0xB, ALL);
        let (_, stat) = run_battle(ctx, Card::assault(0, 0, 0, ALL), defender)?;
        assert_eq!(stat.digit, Digit::PhysicalDefense);
        assert_eq!(stat.value, 0xA);

        // magical defense is lowest
        let defender: Card = Card::physical(0xB, 0xB, 0xA, ALL);
        let (_, stat) = run_battle(ctx, Card::assault(0, 0, 0, ALL), defender)?;
        assert_eq!(stat.digit, Digit::MagicalDefense);
        assert_eq!(stat.value, 0xA);

        // attack is lowest
        let defender: Card = Card::physical(0xA, 0xB, 0xB, ALL);
        let (_, stat) = run_battle(ctx, Card::assault(0, 0, 0, ALL), defender)?;
        assert_eq!(stat.digit, Digit::Attack);
        assert_eq!(stat.value, 0xA);
    });
}

pub(crate) fn run(ctx: Ctx<'_>) {
    if let Ctx::External { implementation } = ctx {
        println!("Running tests on implementation: {}\n", implementation);
    } else {
        println!("Running tests on reference implementation\n",);
    }

    let mut harness = Harness::new(ctx);

    game_setup_tests(suite!(harness "Game Setup"));

    pre_game_tests(suite!(harness "Pre-Game"));

    in_game_tests(suite!(harness "In-Game"));

    let (_, counts) = harness.run();

    counts.print();
}
