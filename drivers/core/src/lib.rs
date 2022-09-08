mod command;
mod driver;
mod response;

pub use command::Command;
pub use driver::ImplementationDriver;
pub use response::{ErrorResponse, Response};

const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;

pub type Seed = u64;
pub type Hand = [Card; HAND_SIZE];
pub type HandCandidates = [Hand; HAND_CANDIDATES];

#[derive(Debug, Clone, PartialEq)]
pub enum Rng {
    Seeded { seed: Seed },
    External { rolls: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BattleSystem {
    Original,
    Dice { sides: u8 },
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Player {
    P1,
    P2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrows(pub u8);

impl Arrows {
    pub const NONE: Arrows = Arrows(0b0000_0000);
    pub const ALL: Arrows = Arrows(0b1111_1111);

    // clockwise from the top
    pub const UP: Arrows = Arrows(0b1000_0000);
    pub const UP_RIGHT: Arrows = Arrows(0b0100_0000);
    pub const RIGHT: Arrows = Arrows(0b0010_0000);
    pub const DOWN_RIGHT: Arrows = Arrows(0b0001_0000);
    pub const DOWN: Arrows = Arrows(0b0000_1000);
    pub const DOWN_LEFT: Arrows = Arrows(0b0000_0100);
    pub const LEFT: Arrows = Arrows(0b0000_0010);
    pub const UP_LEFT: Arrows = Arrows(0b0000_0001);
}

impl std::ops::BitOr for Arrows {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Arrows(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub attack: u8,
    pub card_type: CardType,
    pub physical_defense: u8,
    pub magical_defense: u8,
    pub arrows: Arrows,
}

impl Card {
    pub const fn new(
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

    pub const fn physical(att: u8, phy: u8, mag: u8, arrows: Arrows) -> Self {
        Self::new(att, CardType::Physical, phy, mag, arrows)
    }

    pub const fn magical(att: u8, phy: u8, mag: u8, arrows: Arrows) -> Self {
        Self::new(att, CardType::Magical, phy, mag, arrows)
    }

    pub const fn exploit(att: u8, phy: u8, mag: u8, arrows: Arrows) -> Self {
        Self::new(att, CardType::Exploit, phy, mag, arrows)
    }

    pub const fn assault(att: u8, phy: u8, mag: u8, arrows: Arrows) -> Self {
        Self::new(att, CardType::Assault, phy, mag, arrows)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Event {
    NextTurn {
        to: Player,
    },
    Flip {
        cell: u8,
    },
    ComboFlip {
        cell: u8,
    },
    Battle {
        attacker: Battler,
        defender: Battler,
        winner: BattleWinner,
    },
    GameOver {
        winner: Option<Player>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Battler {
    pub cell: u8,
    pub digit: Digit,
    pub value: u8,
    pub roll: u8,
}

impl Battler {
    pub fn new(cell: u8, digit: Digit, value: u8, roll: u8) -> Self {
        Self {
            cell,
            digit,
            value,
            roll,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Digit {
    Attack,
    PhysicalDefense,
    MagicalDefense,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BattleWinner {
    Attacker,
    Defender,
    None,
}