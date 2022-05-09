const FG_RED: &str = "\x1b[0;31m";
const FG_BLUE: &str = "\x1b[0;34m";
const FG_GRAY: &str = "\x1b[0;30m";
const BG_GRAY: &str = "\x1b[1;30m";
const RESET: &str = "\x1b[0m";

struct GameState {
    turn: Player,
    board: [Cell; 4 * 4],
    p1_hand: [Option<Card>; 5],
    p2_hand: [Option<Card>; 5],
}

#[derive(Clone, Copy, PartialEq)]
enum Player {
    P1,
    P2,
}

#[derive(Clone, Copy)]
enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Clone, Default)]
struct Arrows {
    top_left: bool,
    top: bool,
    top_right: bool,
    left: bool,
    right: bool,
    bottom_left: bool,
    bottom: bool,
    bottom_right: bool,
}

#[derive(Clone)]
struct Card {
    card_type: CardType,
    attack: u8,
    physical_defense: u8,
    magical_defense: u8,
    arrows: Arrows,
}

impl Card {
    fn new(stats: &str, arrows: Arrows) -> Self {
        fn from_hex(num: char) -> u8 {
            match num {
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
                'A' => 10,
                'B' => 11,
                'C' => 12,
                'D' => 13,
                'E' => 14,
                'F' => 15,
                ch => panic!("Invalid hex digit: {ch}"),
            }
        }

        let stats: Vec<char> = stats.chars().collect();
        let attack = from_hex(stats[0]);
        let card_type = match stats[1] {
            'P' => CardType::Physical,
            'M' => CardType::Magical,
            'X' => CardType::Exploit,
            'A' => CardType::Assault,
            ch => panic!("Invalid card type: {ch}"),
        };
        let physical_defense = from_hex(stats[2]);
        let magical_defense = from_hex(stats[3]);
        Card {
            card_type,
            attack,
            physical_defense,
            magical_defense,
            arrows,
        }
    }
}

enum Cell {
    Blocked,
    Card { owner: Player, card: Card },
    Empty,
}

fn render_screen(game_state: &GameState, out: &mut String) {
    fn push_hex_digit(out: &mut String, num: u8) {
        match num {
            0 => out.push('0'),
            1 => out.push('1'),
            2 => out.push('2'),
            3 => out.push('3'),
            4 => out.push('4'),
            5 => out.push('5'),
            6 => out.push('6'),
            7 => out.push('7'),
            8 => out.push('8'),
            9 => out.push('9'),
            10 => out.push('A'),
            11 => out.push('B'),
            12 => out.push('C'),
            13 => out.push('D'),
            14 => out.push('E'),
            15 => out.push('F'),
            _ => unreachable!(),
        }
    }

    fn push_card_stats(out: &mut String, card: &Card) {
        push_hex_digit(out, card.attack);

        use CardType::*;
        match card.card_type {
            Physical => out.push('P'),
            Magical => out.push('M'),
            Exploit => out.push('X'),
            Assault => out.push('A'),
        }

        push_hex_digit(out, card.physical_defense);
        push_hex_digit(out, card.magical_defense);
    }

    fn push_player_color(out: &mut String, player: Player) {
        out.push_str(if player == Player::P1 {
            FG_BLUE
        } else {
            FG_RED
        });
    }

    fn push_hand(out: &mut String, hand: &[Option<Card>; 5]) {
        for (i, card) in hand.iter().enumerate() {
            if card.is_some() {
                out.push_str("╔═══ ");
                push_hex_digit(out, i as u8 + 1);
                out.push_str(" ═══╗");
            }
        }
        out.push('\n');

        let iter = hand.iter().filter_map(Option::as_ref);

        for card in iter.clone() {
            out.push_str("║ ");
            out.push(if card.arrows.top_left { '⇖' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.top { '⇑' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.top_right { '⇗' } else { ' ' });
            out.push_str(" ║");
        }
        out.push('\n');

        for card in iter.clone() {
            out.push_str("║ ");
            out.push(if card.arrows.left { '⇐' } else { ' ' });
            out.push(' ');
            push_card_stats(out, card);
            out.push(if card.arrows.right { '⇒' } else { ' ' });
            out.push_str(" ║");
        }
        out.push('\n');

        for card in iter.clone() {
            out.push_str("║ ");
            out.push(if card.arrows.bottom_left { '⇙' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.bottom { '⇓' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.bottom_right { '⇘' } else { ' ' });
            out.push_str(" ║");
        }
        out.push('\n');

        for _ in iter.clone() {
            out.push_str("╚═════════╝");
        }
        out.push('\n');
    }

    push_player_color(out, Player::P1);
    push_hand(out, &game_state.p1_hand);
    out.push_str(RESET);

    out.push_str("\n   ┌───  1  ───┬───  2  ───┬───  3  ───┬───  4  ───┐\n");

    for (i, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        out.push_str("   │");
        for j in row {
            match &game_state.board[j] {
                Cell::Card { owner, card } => {
                    push_player_color(out, *owner);
                    out.push(' ');
                    out.push(if card.arrows.top_left { '⇖' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.top { '⇑' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.top_right { '⇗' } else { ' ' });
                    out.push(' ');
                    out.push_str(RESET);
                }
                Cell::Blocked => {
                    out.push_str(BG_GRAY);
                    out.push_str(" ╔═══════╗ ");
                    out.push_str(RESET);
                }
                Cell::Empty => out.push_str("           "),
            }
            out.push('│');
        }

        out.push_str("\n    ");
        for j in row {
            if let Cell::Blocked = &game_state.board[j] {
                out.push_str(BG_GRAY);
                out.push_str(" ║       ║ ");
                out.push_str(RESET);
            } else {
                out.push_str("           ");
            }
            out.push('│');
        }
        out.push_str("\n   ");

        push_hex_digit(out, i as u8 + 10); // + 10 so that it prints A, B, C, D

        for j in row {
            match &game_state.board[j] {
                Cell::Card { owner, card } => {
                    push_player_color(out, *owner);
                    out.push(' ');
                    out.push(if card.arrows.left { '⇐' } else { ' ' });
                    out.push_str("  ");
                    push_card_stats(out, card);
                    out.push(' ');
                    out.push(if card.arrows.right { '⇒' } else { ' ' });
                    out.push(' ');
                    out.push_str(RESET);
                }
                Cell::Blocked => {
                    out.push_str(BG_GRAY);
                    out.push_str(" ║ BLOCK ║ ");
                    out.push_str(RESET);
                }
                Cell::Empty => out.push_str("           "),
            }
            out.push('│');
        }

        out.push_str("\n    ");
        for j in row {
            if let Cell::Blocked = &game_state.board[j] {
                out.push_str(BG_GRAY);
                out.push_str(" ║       ║ ");
                out.push_str(RESET);
            } else {
                out.push_str("           ");
            }
            out.push('│');
        }
        out.push_str("\n   │");

        for j in row {
            match &game_state.board[j] {
                Cell::Card { owner, card } => {
                    push_player_color(out, *owner);
                    out.push(' ');
                    out.push(if card.arrows.bottom_left { '⇙' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.bottom { '⇓' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.bottom_right { '⇘' } else { ' ' });
                    out.push(' ');
                    out.push_str(RESET);
                }
                Cell::Blocked => {
                    out.push_str(BG_GRAY);
                    out.push_str(" ╚═══════╝ ");
                    out.push_str(RESET);
                }
                Cell::Empty => out.push_str("           "),
            }
            out.push('│');
        }

        if i != 3 {
            out.push_str("\n   ├───────────┼───────────┼───────────┼───────────┤\n");
        }
    }

    out.push_str("\n   └───────────┴───────────┴───────────┴───────────┘\n\n");

    push_player_color(out, Player::P2);
    push_hand(out, &game_state.p2_hand);
    out.push_str(RESET);

    push_player_color(out, game_state.turn);
    out.push_str("\nPlayer ");
    out.push(if game_state.turn == Player::P1 {
        '1'
    } else {
        '2'
    });
    out.push_str("'s Turn");

    out.push_str(FG_GRAY);
    out.push_str(" [ format: {CARD#} {COORD1}{COORD2} | eg: `1 a3`, `3 2b` ]\n");
    out.push_str(RESET);
}

fn clear_screen(out: &mut String) {
    out.push_str("\x1b]50;ClearScrollback\x07")
}

#[derive(Debug)]
struct Input {
    card: u8,
    cell: u8,
}

fn parse_input(input: &str) -> Result<Input, String> {
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

fn main() {
    let card = Card::new(
        "9P2F",
        Arrows {
            top_left: true,
            top_right: true,
            left: true,
            bottom: true,
            bottom_right: true,
            ..Default::default()
        },
    );
    let mut game_state = GameState {
        turn: Player::P1,
        board: [
            Cell::Card {
                owner: Player::P1,
                card: card.clone(),
            },
            Cell::Blocked,
            Cell::Card {
                owner: Player::P2,
                card: card.clone(),
            },
            Cell::Empty,
            Cell::Blocked,
            Cell::Empty,
            Cell::Empty,
            Cell::Blocked,
            Cell::Empty,
            Cell::Blocked,
            Cell::Empty,
            Cell::Empty,
            Cell::Blocked,
            Cell::Empty,
            Cell::Blocked,
            Cell::Empty,
        ],
        p1_hand: [
            Some(Card::new("1MFF", Default::default())),
            Some(card.clone()),
            Some(Card::new("1PAF", Default::default())),
            Some(Card::new("1P3F", Default::default())),
            Some(Card::new("1MF5", Default::default())),
        ],
        p2_hand: [
            Some(Card::new("1MFF", Default::default())),
            Some(Card::new("2MFF", Default::default())),
            Some(Card::new("3MFF", Default::default())),
            Some(Card::new("4MFF", Default::default())),
            Some(Card::new("5MFF", Default::default())),
        ],
    };

    let mut i = 0;
    let mut j = 2;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let stdin = std::io::stdin();
    let mut in_ = stdin.lock();

    let mut buf = String::new();
    loop {
        use std::io::{BufRead, Write};

        buf.clear();
        clear_screen(&mut buf);
        render_screen(&game_state, &mut buf);
        out.write_all(buf.as_bytes()).unwrap();
        out.flush().unwrap();

        let old_i = i;
        i = (i + 1) % 16;
        game_state.board.swap(old_i, i);

        let old_j = j;
        j = (j + 1) % 16;
        game_state.board.swap(old_j, j);

        let input = loop {
            out.write_all(b"> ").unwrap();
            out.flush().unwrap();

            buf.clear();
            in_.read_line(&mut buf).unwrap();
            match parse_input(&buf) {
                Ok(input) => {
                    break input;
                }
                Err(err) => {
                    println!("ERR: {}", err);
                }
            }
        };

        println!("Input: {:?}", input);
        std::thread::sleep(std::time::Duration::from_millis(1000));
        game_state.turn = match game_state.turn {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
        };
    }
}
