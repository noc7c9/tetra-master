mod display;
mod driver;
mod random_setup;
mod ref_impl;

pub use driver::{Driver, DriverBuilder, Seed};
pub use ref_impl::ReferenceImplementation;

/*****************************************************************************************
 * Command / Response
 */

pub mod command;
pub mod response;

pub use response::ErrorResponse;

pub trait CommandResponse: command::Command {
    type Response: response::Response;
}

impl CommandResponse for command::Setup {
    type Response = response::SetupOk;
}
impl CommandResponse for command::PushRngNumbers {
    type Response = response::PushRngNumbersOk;
}
impl CommandResponse for command::PickHand {
    type Response = response::PickHandOk;
}
impl CommandResponse for command::PlaceCard {
    type Response = response::PlayOk;
}
impl CommandResponse for command::PickBattle {
    type Response = response::PlayOk;
}

/*****************************************************************************************
 * Error Types
 */

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ErrorResponse(response::ErrorResponse),
    SerializationError(command::Error),
    DeserializationError(response::Error),
    IOError(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorResponse(inner) => {
                write!(f, "Got error response: {inner:?}")
            }
            Error::SerializationError(inner) => {
                write!(f, "Failed to serialize response: {inner:?}")
            }
            Error::DeserializationError(inner) => {
                write!(f, "Failed to deserialize response: {inner:?}")
            }
            Error::IOError(inner) => {
                write!(f, "IOError: {inner:?}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::fmt::Error> for Error {
    fn from(inner: std::fmt::Error) -> Self {
        Self::SerializationError(inner)
    }
}

impl From<response::Error> for Error {
    fn from(inner: response::Error) -> Self {
        Self::DeserializationError(inner)
    }
}

impl From<std::io::Error> for Error {
    fn from(inner: std::io::Error) -> Self {
        Self::IOError(inner)
    }
}

/*****************************************************************************************
 * Common Types / Constants
 */

pub const MAX_NUMBER_OF_BLOCKS: u8 = 6;
pub const BOARD_SIZE: usize = 4 * 4;
pub const HAND_CANDIDATES: usize = 3;
pub const HAND_SIZE: usize = 5;

pub type Hand = [Card; HAND_SIZE];
pub type HandCandidates = [Hand; HAND_CANDIDATES];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    numbers: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSystem {
    /// The original system from FF9
    Original,
    /// Dice based system that throws dice based on the stat values
    Dice { sides: u8 },
    /// Non-random system that directly compares the stat values
    Deterministic,
    /// A more predictable system intended for testing
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    P1,
    P2,
}

impl Player {
    pub fn opposite(self) -> Self {
        match self {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    // return true if `self` point in *any* of the directions in `other`
    pub fn has_any(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    // returns an Arrows with all of the arrows pointing in the opposite direction
    pub fn reverse(self) -> Self {
        // wrapping shift by 4 bits
        // this is effectively rotating the arrows by 180 degrees
        Arrows(self.0 >> 4 | self.0 << 4)
    }
}

impl std::ops::BitOr for Arrows {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Arrows(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        debug_assert!(attack <= 0xF, "attack outside expected range 0-F");
        debug_assert!(
            physical_defense <= 0xF,
            "physical defense outside expected range 0-F"
        );
        debug_assert!(
            magical_defense <= 0xF,
            "magical defense outside expected range 0-F"
        );
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Digit {
    Attack,
    PhysicalDefense,
    MagicalDefense,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BattleWinner {
    Attacker,
    Defender,
    None,
}
