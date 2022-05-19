use crate::{GameState, GameStatus, Input, InputBattle, InputPlace};

pub(crate) fn parse(state: &GameState, input: &str) -> Result<Input, String> {
    Ok(match &state.status {
        GameStatus::WaitingPlace => Input::Place(parse_place(input)?),
        GameStatus::WaitingBattle { .. } => Input::Battle(parse_battle(input)?),
        GameStatus::GameOver { .. } => panic!("parse shouldn't be called once game is over"),
    })
}

fn parse_place(input: &str) -> Result<InputPlace, String> {
    enum State {
        ReadingCard,
        ReadingCell { card: usize },
        WaitingForEOL { card: usize, cell: usize },
    }

    let mut state = State::ReadingCard;

    for ch in input.chars() {
        if ch == ' ' {
            continue; // ignore spaces
        }
        match state {
            State::ReadingCard => match char_to_card(ch) {
                Some(card) => state = State::ReadingCell { card },
                _ => return Err(format!("Invalid Card {}", ch)),
            },
            State::ReadingCell { card } => match char_to_cell(ch) {
                Some(cell) => state = State::WaitingForEOL { card, cell },
                _ => return Err(format!("Invalid Cell {}", ch)),
            },
            State::WaitingForEOL { card, cell } => match ch {
                '\n' => return Ok(InputPlace { card, cell }),
                _ => return Err(format!("Unexpected Character {}", ch)),
            },
        }
    }

    unreachable!()
}

fn parse_battle(input: &str) -> Result<InputBattle, String> {
    enum State {
        ReadingCell,
        WaitingForEOL { cell: usize },
    }

    let mut state = State::ReadingCell;

    for ch in input.chars() {
        if ch == ' ' {
            continue; // ignore spaces
        }
        match state {
            State::ReadingCell => match char_to_cell(ch) {
                Some(cell) => state = State::WaitingForEOL { cell },
                _ => return Err(format!("Invalid Cell {}", ch)),
            },
            State::WaitingForEOL { cell } => match ch {
                '\n' => return Ok(InputBattle { cell }),
                _ => return Err(format!("Unexpected Character {}", ch)),
            },
        }
    }

    unreachable!()
}

fn char_to_card(ch: char) -> Option<usize> {
    Some(match ch {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        _ => return None,
    })
}

fn char_to_cell(ch: char) -> Option<usize> {
    Some(match ch {
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
        _ => return None,
    })
}
