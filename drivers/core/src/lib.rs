mod display;
mod driver;
mod random_setup;
mod ref_impl;

mod command;
mod response;

pub use driver::{Driver, DriverBuilder, Seed};
pub use ref_impl::ReferenceImplementation;

/*****************************************************************************************
 * Command / Response
 */

pub use command::{PickBattle, PlaceCard, ResolveBattle, Setup};
pub use response::{ErrorResponse, PlayOk, SetupOk};

pub trait CommandResponse: command::Command {
    type Response: response::Response;
}

impl CommandResponse for command::Setup {
    type Response = response::SetupOk;
}
impl CommandResponse for command::PlaceCard {
    type Response = response::PlayOk;
}
impl CommandResponse for command::PickBattle {
    type Response = response::PlayOk;
}
impl CommandResponse for command::ResolveBattle {
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
pub const HAND_SIZE: usize = 5;

pub type Hand = [Card; HAND_SIZE];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSystem {
    /// The original system from FF9
    Original,
    /// Dice based system that throws dice based on the stat values
    Dice { sides: u8 },
    /// Non-random system that directly compares the stat values
    Deterministic,
    /// A system entirely controlled externally (ignores the stat, uses roll directly)
    /// intended for testing
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Blue,
    Red,
}

impl Player {
    pub fn opposite(self) -> Self {
        match self {
            Player::Blue => Player::Red,
            Player::Red => Player::Blue,
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

    // intended only for testing so fn panics instead of returning an error
    pub fn parse(s: &str) -> Self {
        Card::new(
            u8::from_str_radix(&s[0..1], 16).unwrap(),
            match &s[1..2] {
                "P" => CardType::Physical,
                "M" => CardType::Magical,
                "X" => CardType::Exploit,
                "A" => CardType::Assault,
                _ => unreachable!(),
            },
            u8::from_str_radix(&s[2..3], 16).unwrap(),
            u8::from_str_radix(&s[3..4], 16).unwrap(),
            Arrows(u8::from_str_radix(&s[5..], 16).unwrap()),
        )
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

// A type representing a selection of board cells implemented as a bitset
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BoardCells(pub u16);

impl BoardCells {
    pub const NONE: BoardCells = BoardCells(0b0000_0000_0000_0000);
    pub const ALL: BoardCells = BoardCells(0b1111_1111_1111_1111);

    pub fn has(&self, cell: u8) -> bool {
        debug_assert!(cell < 16);
        let cell = 1 << cell as u16;
        self.0 & cell != 0
    }

    pub fn set(&mut self, cell: u8) {
        debug_assert!(cell < 16);
        let cell = 1 << cell as u16;
        self.0 |= cell;
    }

    pub fn unset(&mut self, cell: u8) {
        debug_assert!(cell < 16);
        let cell = 1 << cell as u16;
        self.0 &= !cell;
    }

    pub fn len(&self) -> usize {
        self.into_iter().count()
    }

    pub fn is_empty(&self) -> bool {
        *self == BoardCells::NONE
    }
}

impl Default for BoardCells {
    fn default() -> Self {
        BoardCells::NONE
    }
}

impl From<&[u8]> for BoardCells {
    fn from(cells: &[u8]) -> Self {
        debug_assert!(cells.len() <= 16);
        let mut board_cells = BoardCells::NONE;
        for cell in cells {
            board_cells.set(*cell);
        }
        board_cells
    }
}

macro_rules! impl_for_array {
    ($len:expr) => {
        impl From<[u8; $len]> for BoardCells {
            fn from(cells: [u8; $len]) -> Self {
                cells.as_slice().into()
            }
        }
    };
}

impl_for_array!(0);
impl_for_array!(1);
impl_for_array!(2);
impl_for_array!(3);
impl_for_array!(4);
impl_for_array!(5);
impl_for_array!(6);
impl_for_array!(7);
impl_for_array!(8);
impl_for_array!(9);
impl_for_array!(10);
impl_for_array!(11);
impl_for_array!(12);
impl_for_array!(13);
impl_for_array!(14);
impl_for_array!(15);

impl From<Vec<u8>> for BoardCells {
    fn from(cells: Vec<u8>) -> Self {
        cells.as_slice().into()
    }
}

impl IntoIterator for BoardCells {
    type Item = u8;
    type IntoIter = BoardCellsIter;
    fn into_iter(self) -> BoardCellsIter {
        BoardCellsIter::new(self)
    }
}

pub struct BoardCellsIter {
    cells: BoardCells,
    index: u8,
}

impl BoardCellsIter {
    fn new(cells: BoardCells) -> Self {
        Self { cells, index: 0 }
    }
}

impl Iterator for BoardCellsIter {
    type Item = u8;
    fn next(&mut self) -> std::option::Option<u8> {
        while self.index < 16 {
            if self.cells.has(self.index) {
                let next = Some(self.index);
                self.index += 1;
                return next;
            }
            self.index += 1;
        }
        None
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
