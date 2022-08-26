mod driver;
mod test_harness;

const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;

type Seed = u64;
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];

#[derive(Debug, Clone, PartialEq)]
enum BattleSystem {
    Original,
    Dice { sides: u8 },
    External { rolls: Vec<u8> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Arrows(u8);

impl Arrows {
    const ALL: Arrows = Arrows(0b1111_1111);

    const UP: Arrows = Arrows(0b1000_0000);

    const fn new(bitset: u8) -> Self {
        Self(bitset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Card {
    attack: u8,
    card_type: CardType,
    physical_defense: u8,
    magical_defense: u8,
    arrows: Arrows,
}

impl Card {
    const fn new(
        attack: u8,
        card_type: CardType,
        physical_defense: u8,
        magical_defense: u8,
        arrows: Arrows,
    ) -> Self {
        Self {
            attack,
            card_type,
            physical_defense,
            magical_defense,
            arrows,
        }
    }

    // shortcut constructors
    const fn physical(att: u8, phy: u8, mag: u8, arrows: u8) -> Self {
        Self::new(att, CardType::Physical, phy, mag, Arrows::new(arrows))
    }
    const fn magical(att: u8, phy: u8, mag: u8, arrows: u8) -> Self {
        Self::new(att, CardType::Magical, phy, mag, Arrows::new(arrows))
    }
    const fn exploit(att: u8, phy: u8, mag: u8, arrows: u8) -> Self {
        Self::new(att, CardType::Exploit, phy, mag, Arrows::new(arrows))
    }
    const fn assault(att: u8, phy: u8, mag: u8, arrows: u8) -> Self {
        Self::new(att, CardType::Assault, phy, mag, Arrows::new(arrows))
    }
}

struct ImplementationDriver {
    proc: std::process::Child,
    driver: driver::Driver<std::io::BufReader<std::process::ChildStdout>, std::process::ChildStdin>,
    _stderr_thread_handle: std::thread::JoinHandle<()>,
}

impl ImplementationDriver {
    fn send(&mut self, cmd: driver::Command) -> anyhow::Result<driver::Response> {
        self.driver.send(cmd)
    }

    #[allow(dead_code)]
    fn toggle_logging(&mut self) {
        self.driver.toggle_logging()
    }
}

impl Drop for ImplementationDriver {
    fn drop(&mut self) {
        // if killing the child fails, just ignore it
        // the OS should clean up after the tester process closes
        let _ = self.proc.kill();
    }
}

fn implementation_driver(implementation: &str) -> ImplementationDriver {
    use std::process::{Command, Stdio};

    let mut proc = Command::new(implementation)
        .args(["--headless"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = proc.stdin.take().unwrap();

    let stdout = proc.stdout.take().unwrap();
    let stdout = std::io::BufReader::new(stdout);

    // manually handle letting stderr passthrough to ensure output from the driver and the
    // implementation don't get mixed up (at least in the middle of a line)
    let stderr = proc.stderr.take().unwrap();
    let stderr = std::io::BufReader::new(stderr);
    let handle = std::thread::spawn(move || {
        use std::io::BufRead;
        for line in stderr.lines() {
            eprintln!("{}", line.unwrap());
        }
    });

    let driver = driver::Driver::new(stdout, stdin);

    ImplementationDriver {
        proc,
        driver,
        _stderr_thread_handle: handle,
    }
}

#[derive(Debug, clap::Parser)]
struct Args {
    implementation: String,
}

fn main() -> anyhow::Result<()> {
    use driver::{BattleWinner, Battler, Command, Digit, Interaction, Response};
    use pretty_assertions::{assert_eq, assert_ne};
    use test_harness::Harness;

    let args = {
        use clap::Parser;
        Args::parse()
    };

    let mut harness = Harness::new();

    // reused mock values

    const C0P00_0: Card = Card::physical(0, 0, 0, 0);
    const C1P23_4: Card = Card::physical(1, 2, 3, 4);
    const C5M67_8: Card = Card::magical(5, 6, 7, 8);
    const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, 0xC);
    const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, 0);

    const HAND_CANDIDATES: HandCandidates = [
        [C5M67_8, CDAEF_0, C9XAB_C, C5M67_8, C1P23_4],
        [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
        [C1P23_4, C5M67_8, CDAEF_0, C5M67_8, C9XAB_C],
    ];

    // setup process
    harness.test("Setup without args", || {
        let mut driver1 = implementation_driver(&args.implementation);
        let first = driver1.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;

        let mut driver2 = implementation_driver(&args.implementation);
        let second = driver2.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;

        assert_ne!(first, second);

        Ok(())
    });

    harness.test("Setup with set seed", || {
        let mut driver1 = implementation_driver(&args.implementation);
        let first = driver1.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;

        let seed = if let Response::SetupOk { seed, .. } = first {
            seed
        } else {
            panic!("unexpected response");
        };

        let mut driver1 = implementation_driver(&args.implementation);
        let second = driver1.send(Command::Setup {
            seed: Some(seed),
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;

        assert_eq!(first, second);

        Ok(())
    });

    harness.test("Setup with set blocked_cells", || {
        let mut driver = implementation_driver(&args.implementation);

        if let Response::SetupOk {
            mut blocked_cells, ..
        } = driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![6u8, 3, 0xC]),
            hand_candidates: None,
        })? {
            blocked_cells.sort_unstable();
            assert_eq!(blocked_cells, vec![3, 6, 0xC]);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("Setup with set blocked_cells to nothing", || {
        let mut driver = implementation_driver(&args.implementation);

        if let Response::SetupOk {
            mut blocked_cells, ..
        } = driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![]),
            hand_candidates: None,
        })? {
            blocked_cells.sort_unstable();
            assert_eq!(blocked_cells, vec![]);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("Setup with set hand candidates", || {
        let expected = HAND_CANDIDATES;
        let mut driver = implementation_driver(&args.implementation);

        if let Response::SetupOk {
            hand_candidates, ..
        } = driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(expected),
        })? {
            assert_eq!(expected, hand_candidates);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    // pre-game
    harness.test("P1 hand selection, ok", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        if let Response::PickHandOk = driver.send(Command::PickHand { index: 1 })? {
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("P1 hand selection, invalid number", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        if let Response::PickHandErr { reason } = driver.send(Command::PickHand { index: 3 })? {
            assert_eq!(reason, "Invalid Pick '3', expected a number from 0 to 2");
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("P2 hand selection, ok", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        driver.send(Command::PickHand { index: 0 })?;
        if let Response::PickHandOk = driver.send(Command::PickHand { index: 1 })? {
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("P2 hand selection, invalid number", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        driver.send(Command::PickHand { index: 0 })?;
        if let Response::PickHandErr { reason } = driver.send(Command::PickHand { index: 3 })? {
            assert_eq!(reason, "Invalid Pick '3', expected a number from 0 to 2");
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("P2 hand selection, hand already selected", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        driver.send(Command::PickHand { index: 0 })?;
        if let Response::PickHandErr { reason } = driver.send(Command::PickHand { index: 0 })? {
            assert_eq!(reason, "Hand 0 has already been picked");
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    // game proper
    harness.test("place card on empty board", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![]),
            hand_candidates: Some(HAND_CANDIDATES),
        })?;

        driver.send(Command::PickHand { index: 0 })?;
        if let Response::PickHandErr { reason } = driver.send(Command::PickHand { index: 0 })? {
            assert_eq!(reason, "Hand 0 has already been picked");
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card with no interaction", || {
        let mut driver = implementation_driver(&args.implementation);
        let hand_candidates = [
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 1, cell: 5 })?
        {
            assert_eq!(interactions, vec![]);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that flips one other card", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender = Card::physical(0, 0, 0, 0);
        let attacker = Card::physical(0, 0, 0, Arrows::UP.0);
        let hand_candidates = [
            [C0P00_0, C0P00_0, defender, C0P00_0, C0P00_0],
            [C0P00_0, attacker, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;
        driver.send(Command::PlaceCard { card: 2, cell: 1 })?;

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 1, cell: 5 })?
        {
            assert_eq!(interactions, vec![Interaction::Flip { cell: 1 }]);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that flips multiple other cards", || {
        let mut driver = implementation_driver(&args.implementation);
        let attacker = Card::physical(0, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, attacker],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: None,
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 1 })?;
        driver.send(Command::PlaceCard { card: 0, cell: 2 })?; // att
        driver.send(Command::PlaceCard { card: 1, cell: 6 })?;
        driver.send(Command::PlaceCard { card: 1, cell: 0xA })?; // att
        driver.send(Command::PlaceCard { card: 2, cell: 4 })?;
        driver.send(Command::PlaceCard { card: 2, cell: 8 })?; // att
        driver.send(Command::PlaceCard { card: 3, cell: 0 })?;

        if let Response::PlaceCardOk { mut interactions } =
            driver.send(Command::PlaceCard { card: 4, cell: 5 })?
        {
            interactions.sort_unstable_by_key(|int| match int {
                Interaction::Flip { cell } => *cell,
                _ => unreachable!(),
            });
            assert_eq!(
                interactions,
                vec![
                    Interaction::Flip { cell: 0 },
                    Interaction::Flip { cell: 1 },
                    Interaction::Flip { cell: 4 },
                    Interaction::Flip { cell: 6 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that results in a battle, attacker wins", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [attacker, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: Some(BattleSystem::External {
                rolls: vec![123, 45],
            }),
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 0 })?;

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 0, cell: 1 })?
        {
            assert_eq!(
                interactions,
                vec![
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 1,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 123
                        },
                        defender: Battler {
                            cell: 0,
                            digit: Digit::PhysicalDefense,
                            value: 3,
                            roll: 45
                        },
                        winner: BattleWinner::Attacker,
                    },
                    Interaction::Flip { cell: 0 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that results in a battle, defender wins", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [attacker, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: Some(BattleSystem::External {
                rolls: vec![12, 35],
            }),
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 0 })?;

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 0, cell: 1 })?
        {
            assert_eq!(
                interactions,
                vec![
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 1,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 12
                        },
                        defender: Battler {
                            cell: 0,
                            digit: Digit::PhysicalDefense,
                            value: 3,
                            roll: 35
                        },
                        winner: BattleWinner::Defender,
                    },
                    Interaction::Flip { cell: 1 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that results in a battle, draw", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [attacker, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: Some(BattleSystem::External {
                rolls: vec![12, 12],
            }),
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 0 })?;

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 0, cell: 1 })?
        {
            assert_eq!(
                interactions,
                vec![
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 1,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 12
                        },
                        defender: Battler {
                            cell: 0,
                            digit: Digit::PhysicalDefense,
                            value: 3,
                            roll: 12
                        },
                        winner: BattleWinner::None,
                    },
                    Interaction::Flip { cell: 1 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that results in a combo", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender = Card::physical(0, 3, 7, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [attacker, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: Some(BattleSystem::External { rolls: vec![2, 1] }),
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 5 })?; // defender
        driver.send(Command::PlaceCard { card: 1, cell: 0xF })?; // out of the way
        driver.send(Command::PlaceCard { card: 1, cell: 0 })?; // will be combo'd

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PlaceCard { card: 0, cell: 9 })?
        {
            assert_eq!(
                interactions,
                vec![
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 9,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 2
                        },
                        defender: Battler {
                            cell: 5,
                            digit: Digit::PhysicalDefense,
                            value: 3,
                            roll: 1
                        },
                        winner: BattleWinner::Attacker,
                    },
                    Interaction::Flip { cell: 5 },
                    Interaction::ComboFlip { cell: 0 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("place card that results in a choice", || {
        let mut driver = implementation_driver(&args.implementation);
        let defender1 = Card::physical(0, 3, 7, Arrows::ALL.0);
        let defender2 = Card::physical(0, 9, 4, Arrows::ALL.0);
        let attacker = Card::exploit(0xC, 0, 0, Arrows::ALL.0);
        let hand_candidates = [
            [defender1, defender2, C0P00_0, C0P00_0, C0P00_0],
            [attacker, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
            [C0P00_0, C0P00_0, C0P00_0, C0P00_0, C0P00_0],
        ];
        driver.send(Command::Setup {
            seed: None,
            battle_system: Some(BattleSystem::External {
                rolls: vec![2, 1, 12, 23],
            }),
            blocked_cells: Some(vec![]),
            hand_candidates: Some(hand_candidates),
        })?;
        driver.send(Command::PickHand { index: 0 })?;
        driver.send(Command::PickHand { index: 1 })?;

        driver.send(Command::PlaceCard { card: 0, cell: 0 })?; // defender 1
        driver.send(Command::PlaceCard { card: 1, cell: 0xF })?; // out of the way
        driver.send(Command::PlaceCard { card: 1, cell: 8 })?; // defender 2

        if let Response::PlaceCardPickBattle { mut choices } =
            driver.send(Command::PlaceCard { card: 0, cell: 4 })?
        {
            choices.sort_unstable();
            assert_eq!(choices, vec![0, 8]);
        } else {
            panic!("unexpected response");
        }

        if let Response::PlaceCardOk { interactions } =
            driver.send(Command::PickBattle { cell: 8 })?
        {
            assert_eq!(
                interactions,
                vec![
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 4,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 2
                        },
                        defender: Battler {
                            cell: 8,
                            digit: Digit::MagicalDefense,
                            value: 4,
                            roll: 1
                        },
                        winner: BattleWinner::Attacker,
                    },
                    Interaction::Flip { cell: 8 },
                    Interaction::Battle {
                        attacker: Battler {
                            cell: 4,
                            digit: Digit::Attack,
                            value: 0xC,
                            roll: 12
                        },
                        defender: Battler {
                            cell: 0,
                            digit: Digit::PhysicalDefense,
                            value: 3,
                            roll: 23
                        },
                        winner: BattleWinner::Defender,
                    },
                    Interaction::Flip { cell: 4 },
                    Interaction::ComboFlip { cell: 8 },
                ]
            );
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    println!("Running tests...\n");
    harness.run();

    Ok(())
}
