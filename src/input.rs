#[derive(Debug)]
pub(crate) struct Input {
    card: u8,
    cell: u8,
}

pub(crate) fn parse(input: &str) -> Result<Input, String> {
    enum State {
        ReadingCard,
        ReadingCoord1 {
            card: u8,
        },
        ReadingCoord2 {
            card: u8,
            row: Option<u8>,
            col: Option<u8>,
        },
    }

    fn ch_to_col(ch: char) -> u8 {
        match ch {
            '1' => 0,
            '2' => 1,
            '3' => 2,
            '4' => 3,
            _ => unreachable!(),
        }
    }
    fn ch_to_row(ch: char) -> u8 {
        match ch {
            'a' | 'A' => 0,
            'b' | 'B' => 1,
            'c' | 'C' => 2,
            'd' | 'D' => 3,
            _ => unreachable!(),
        }
    }

    let mut state = State::ReadingCard;

    for ch in input.chars() {
        if ch == ' ' {
            continue; // ignore spaces
        }
        match state {
            State::ReadingCard => match ch {
                '1' => state = State::ReadingCoord1 { card: 0 },
                '2' => state = State::ReadingCoord1 { card: 1 },
                '3' => state = State::ReadingCoord1 { card: 2 },
                '4' => state = State::ReadingCoord1 { card: 3 },
                '5' => state = State::ReadingCoord1 { card: 4 },
                _ => return Err(format!("Invalid Card {}", ch)),
            },
            State::ReadingCoord1 { card } => match ch {
                '1' | '2' | '3' | '4' => {
                    state = State::ReadingCoord2 {
                        card,
                        row: None,
                        col: Some(ch_to_col(ch)),
                    }
                }
                'a' | 'A' | 'b' | 'B' | 'c' | 'C' | 'd' | 'D' => {
                    state = State::ReadingCoord2 {
                        card,
                        row: Some(ch_to_row(ch)),
                        col: None,
                    }
                }
                _ => return Err(format!("Invalid Coord {}", ch)),
            },
            State::ReadingCoord2 {
                card,
                row,
                col: None,
            } => match ch {
                '1' | '2' | '3' | '4' => {
                    state = State::ReadingCoord2 {
                        card,
                        row,
                        col: Some(ch_to_col(ch)),
                    }
                }
                'a' | 'A' | 'b' | 'B' | 'c' | 'C' | 'd' | 'D' => {
                    return Err("Row defined twice".into())
                }
                _ => return Err(format!("Invalid Coord {}", ch)),
            },
            State::ReadingCoord2 {
                card,
                row: None,
                col,
            } => match ch {
                'a' | 'A' | 'b' | 'B' | 'c' | 'C' | 'd' | 'D' => {
                    state = State::ReadingCoord2 {
                        card,
                        col,
                        row: Some(ch_to_row(ch)),
                    }
                }
                '1' | '2' | '3' | '4' => return Err("Col defined twice".into()),
                _ => return Err(format!("Invalid Coord {}", ch)),
            },
            State::ReadingCoord2 {
                card,
                row: Some(row),
                col: Some(col),
            } => match ch {
                '\n' => {
                    return Ok(Input {
                        card,
                        cell: row * 4 + col,
                    })
                }
                _ => return Err(format!("Unexpected Character {}", ch)),
            },
        }
    }

    unreachable!()
}
