use crate::{GameState, GameStatus, Input, InputBattle, InputPlace};

#[derive(Debug, PartialEq)]
pub(crate) enum Error {
    EmptyInput,
    InvalidCard { ch: char },
    InvalidCell { ch: char },
    MissingCell,
    UnexpectedCharacter { ch: char },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EmptyInput => write!(f, "Empty Input"),
            Error::InvalidCard { ch } => {
                write!(f, "Invalid Card {ch:?}, expected a number from 0 to 4")
            }
            Error::InvalidCell { ch } => {
                write!(f, "Invalid Cell {ch:?}, expected a hex number from 0 to F")
            }
            Error::MissingCell => write!(f, "Missing Cell, expected a hex number from 0 to F"),
            Error::UnexpectedCharacter { ch } => write!(f, "Unexpected Character {ch:?}"),
        }
    }
}

pub(crate) fn parse(state: &GameState, input: &str) -> Result<Input, Error> {
    Ok(match &state.status {
        GameStatus::WaitingPlace => Input::Place(parse_place(input)?),
        GameStatus::WaitingBattle { .. } => Input::Battle(parse_battle(input)?),
        GameStatus::GameOver { .. } => panic!("parse shouldn't be called once game is over"),
    })
}

fn parse_place(input: &str) -> Result<InputPlace, Error> {
    let mut chars = input.chars().filter(|ch| !ch.is_ascii_whitespace());

    // read and parse the (hand) card index
    let card = match chars.next() {
        Some(ch) => char_to_card(ch)?,
        None => return Err(Error::EmptyInput),
    };

    // read and parse the cell index
    let cell = match chars.next() {
        Some(cell) => char_to_cell(cell)?,
        None => return Err(Error::MissingCell),
    };

    if let Some(ch) = chars.next() {
        return Err(Error::UnexpectedCharacter { ch });
    }

    Ok(InputPlace { card, cell })
}

fn parse_battle(input: &str) -> Result<InputBattle, Error> {
    let mut chars = input.chars().filter(|ch| !ch.is_ascii_whitespace());

    // read and parse the cell index
    let cell = match chars.next() {
        Some(cell) => char_to_cell(cell)?,
        None => return Err(Error::EmptyInput),
    };

    if let Some(ch) = chars.next() {
        return Err(Error::UnexpectedCharacter { ch });
    }

    Ok(InputBattle { cell })
}

fn char_to_card(ch: char) -> Result<usize, Error> {
    Ok(match ch {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        _ => return Err(Error::InvalidCard { ch }),
    })
}

fn char_to_cell(ch: char) -> Result<usize, Error> {
    Ok(match ch {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' | 'A' => 10,
        'b' | 'B' => 11,
        'c' | 'C' => 12,
        'd' | 'D' => 13,
        'e' | 'E' => 14,
        'f' | 'F' => 15,
        _ => return Err(Error::InvalidCell { ch }),
    })
}

#[cfg(test)]
mod test_parse_place {
    use super::*;

    #[test]
    fn parse_minimal_input() {
        let res = parse_place("3b");
        assert_eq!(
            res,
            Ok(InputPlace {
                card: 0x3,
                cell: 0xb
            })
        );
    }

    #[test]
    fn ignore_leading_trailing_and_infix_whitespace() {
        let res = parse_place("   4 \t  8  \t \n\t   ");
        assert_eq!(
            res,
            Ok(InputPlace {
                card: 0x4,
                cell: 0x8
            })
        );
    }

    #[test]
    fn error_on_empty_input() {
        let res = parse_place("");
        assert_eq!(res, Err(Error::EmptyInput));
    }

    #[test]
    fn error_when_input_is_all_whitespace() {
        let res = parse_place("  \t     ");
        assert_eq!(res, Err(Error::EmptyInput));
    }

    #[test]
    fn error_when_card_is_invalid() {
        let res = parse_place("a 0");
        assert_eq!(res, Err(Error::InvalidCard { ch: 'a' }));
    }

    #[test]
    fn error_when_cell_is_invalid() {
        let res = parse_place("0 g");
        assert_eq!(res, Err(Error::InvalidCell { ch: 'g' }));
    }

    #[test]
    fn error_when_cell_is_missing() {
        let res = parse_place("0");
        assert_eq!(res, Err(Error::MissingCell));
    }

    #[test]
    fn error_when_there_are_unexpected_characters() {
        let res = parse_place("0a  7");
        assert_eq!(res, Err(Error::UnexpectedCharacter { ch: '7' }));
    }
}

#[cfg(test)]
mod test_parse_battle {
    use super::*;

    #[test]
    fn parse_minimal_input() {
        let res = parse_battle("b");
        assert_eq!(res, Ok(InputBattle { cell: 0xb }));
    }

    #[test]
    fn ignore_leading_and_trailing_whitespace() {
        let res = parse_battle("   4\t \t   ");
        assert_eq!(res, Ok(InputBattle { cell: 0x4 }));
    }

    #[test]
    fn error_on_empty_input() {
        let res = parse_battle("");
        assert_eq!(res, Err(Error::EmptyInput));
    }

    #[test]
    fn error_when_input_is_all_whitespace() {
        let res = parse_battle("  \n\t     ");
        assert_eq!(res, Err(Error::EmptyInput));
    }

    #[test]
    fn error_when_cell_is_invalid() {
        let res = parse_battle("g");
        assert_eq!(res, Err(Error::InvalidCell { ch: 'g' }));
    }

    #[test]
    fn error_when_there_are_unexpected_characters() {
        let res = parse_battle("a  7");
        assert_eq!(res, Err(Error::UnexpectedCharacter { ch: '7' }));
    }
}
