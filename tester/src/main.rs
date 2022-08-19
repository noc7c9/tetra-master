mod driver;
mod test_harness;

const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;

type Seed = u64;
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];

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
}

impl ImplementationDriver {
    fn transmit(&mut self, cmd: driver::Command) -> anyhow::Result<()> {
        self.driver.transmit(cmd)
    }

    fn receive(&mut self) -> anyhow::Result<driver::Response> {
        self.driver.receive()
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
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdin = proc.stdin.take().unwrap();
    let stdout = proc.stdout.take().unwrap();
    let stdout = std::io::BufReader::new(stdout);

    let driver = driver::Driver::new(stdout, stdin);

    ImplementationDriver { proc, driver }
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
        let mut driver1 = implementation_driver(&args.implementation);
        driver1.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let first = driver1.receive()?;

        let mut driver2 = implementation_driver(&args.implementation);
        driver2.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let second = driver2.receive()?;

        assert_ne!(first, second);

        Ok(())
    });

    harness.test("Setup with set seed", || {
        let mut driver1 = implementation_driver(&args.implementation);
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

        let mut driver1 = implementation_driver(&args.implementation);
        driver1.transmit(Command::Setup {
            seed: Some(seed),
            blocked_cells: None,
            hand_candidates: None,
        })?;
        let second = driver1.receive()?;

        assert_eq!(first, second);

        Ok(())
    });

    harness.test("Setup with set blocked_cells", || {
        let mut driver = implementation_driver(&args.implementation);
        driver.transmit(Command::Setup {
            seed: None,
            blocked_cells: Some((&[6u8, 3, 0xC] as &[_]).try_into().unwrap()),
            hand_candidates: None,
        })?;

        if let Response::SetupOk {
            mut blocked_cells, ..
        } = driver.receive()?
        {
            blocked_cells.sort_unstable();
            assert_eq!(blocked_cells.as_slice(), &[3, 6, 0xC]);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    harness.test("Setup with set hand candidates", || {
        const C1P23_4: Card = Card::physical(1, 2, 3, 4);
        const C5M67_8: Card = Card::magical(5, 6, 7, 8);
        const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, 0xC);
        const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, 0);
        let expected = [
            [C5M67_8, CDAEF_0, C9XAB_C, C5M67_8, C1P23_4],
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C1P23_4, C5M67_8, CDAEF_0, C5M67_8, C9XAB_C],
        ];

        let mut driver = implementation_driver(&args.implementation);
        driver.transmit(Command::Setup {
            seed: None,
            blocked_cells: None,
            hand_candidates: Some(expected),
        })?;

        if let Response::SetupOk {
            hand_candidates, ..
        } = driver.receive()?
        {
            assert_eq!(expected, hand_candidates);
            Ok(())
        } else {
            panic!("unexpected response");
        }
    });

    // TODO pre-game

    // TODO game proper

    println!("Running tests...\n");
    harness.run();

    Ok(())
}
