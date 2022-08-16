mod driver;
mod test_harness;

const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;

type Seed = u64;
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Arrows(u8);

impl Arrows {
    pub(crate) const fn new(bitset: u8) -> Self {
        Self(bitset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Card {
    attack: u8,
    card_type: CardType,
    physical_defense: u8,
    magical_defense: u8,
    arrows: Arrows,
}

impl Card {
    pub(crate) const fn new(
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
}

fn implementation_driver(
    implementation: &str,
) -> (
    std::process::Child,
    driver::Driver<std::io::BufReader<std::process::ChildStdout>, std::process::ChildStdin>,
) {
    use std::process::{Command, Stdio};

    let mut proc = Command::new(implementation)
        .args(["--headless"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdin = proc.stdin.take().unwrap();
    let stdout = proc.stdout.take().unwrap();
    let stdout = std::io::BufReader::new(stdout);

    let driver = driver::Driver::new(stdout, stdin);

    (proc, driver)
}

#[derive(Debug, clap::Parser)]
struct Args {
    implementation: String,
}

fn main() -> anyhow::Result<()> {
    use driver::{Command, Response};
    use pretty_assertions::{assert_eq, assert_ne};
    use test_harness::Harness;

    let args = {
        use clap::Parser;
        Args::parse()
    };

    let mut harness = Harness::new();

    // setup process
    harness.test("Setup without args", || {
        let (mut proc1, mut driver1) = implementation_driver(&args.implementation);
        driver1.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let first = driver1.receive()?;

        let (mut proc2, mut driver2) = implementation_driver(&args.implementation);
        driver2.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let second = driver2.receive()?;

        assert_ne!(first, second);

        proc1.kill()?;
        proc2.kill()?;

        Ok(())
    });

    harness.test("Setup with set seed", || {
        let (mut proc1, mut driver1) = implementation_driver(&args.implementation);
        driver1.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let first = driver1.receive()?;

        let seed = if let Response::SetupOk { seed, .. } = first {
            seed
        } else {
            panic!("unexpected response");
        };

        let (mut proc2, mut driver1) = implementation_driver(&args.implementation);
        driver1.transmit(Command::Setup {
            seed: Some(seed),
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let second = driver1.receive()?;

        assert_eq!(first, second);

        proc1.kill()?;
        proc2.kill()?;

        Ok(())
    });

    harness.test("Setup with set blocked_cells", || {
        let (mut proc, mut driver) = implementation_driver(&args.implementation);
        driver.transmit(Command::Setup {
            seed: None,
            blocked_cells: Some((&[6u8, 3, 0xC] as &[_]).try_into().unwrap()),
            hand_candidates: None,
        })?;
        let res = driver.receive()?;

        let mut blocked_cells = if let Response::SetupOk { blocked_cells, .. } = res {
            blocked_cells
        } else {
            panic!("unexpected response");
        };

        blocked_cells.sort_unstable();
        assert_eq!(blocked_cells.as_slice(), &[3, 6, 0xC]);

        proc.kill()?;

        Ok(())
    });

    harness.test("Setup with set hand candidates", || {
        const C1P23_4: Card = Card::new(1, CardType::Physical, 2, 3, Arrows::new(4));
        const C5M67_8: Card = Card::new(5, CardType::Magical, 6, 7, Arrows::new(8));
        const C9XAB_C: Card = Card::new(9, CardType::Exploit, 0xA, 0xB, Arrows::new(0xC));
        const CDAEF_0: Card = Card::new(0xD, CardType::Assault, 0xE, 0xF, Arrows::new(0));
        let expected = [
            [C5M67_8, CDAEF_0, C9XAB_C, C5M67_8, C1P23_4],
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C1P23_4, C5M67_8, CDAEF_0, C5M67_8, C9XAB_C],
        ];

        let (mut proc, mut driver) = implementation_driver(&args.implementation);
        driver.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: Some(expected),
        })?;
        let res = driver.receive()?;

        let hand_candidates = if let Response::SetupOk {
            hand_candidates, ..
        } = res
        {
            hand_candidates
        } else {
            panic!("unexpected response");
        };

        assert_eq!(expected, hand_candidates);

        proc.kill()?;

        Ok(())
    });

    // TODO pre-game

    // TODO game proper

    println!("Running tests...");
    harness.run();
    println!("Done!");

    Ok(())
}
