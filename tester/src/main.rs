mod driver;
#[macro_use]
mod harness;

mod impl_tests;

const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;

type Seed = u64;
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];

#[derive(Debug, Clone, PartialEq)]
enum Rng {
    Seeded { seed: Seed },
    External { rolls: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq)]
enum BattleSystem {
    Original,
    OriginalApprox,
    Dice { sides: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Player {
    P1,
    P2,
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
    const RIGHT: Arrows = Arrows(0b0010_0000);
    const DOWN: Arrows = Arrows(0b0000_1000);
    const LEFT: Arrows = Arrows(0b0000_0010);
    const UP_LEFT: Arrows = Arrows(0b0000_0001);

    const fn new(bitset: u8) -> Self {
        Self(bitset)
    }
}

impl std::ops::BitOr for Arrows {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Arrows(self.0 | rhs.0)
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

#[derive(Debug, clap::Parser)]
struct Args {
    implementation: String,
}

fn main() -> anyhow::Result<()> {
    let args = {
        use clap::Parser;
        Args::parse()
    };

    impl_tests::run(args.implementation);

    Ok(())
}
