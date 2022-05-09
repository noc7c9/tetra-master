use super::{Card, CardType, Cell, GameState, Player};

const FG_RED: &str = "\x1b[0;31m";
const FG_BLUE: &str = "\x1b[0;34m";
const FG_GRAY: &str = "\x1b[0;30m";
const BG_GRAY: &str = "\x1b[1;30m";
const RESET: &str = "\x1b[0m";

pub(crate) fn screen(game_state: &GameState, out: &mut String) {
    // clear screen first thing
    out.push_str("\x1b]50;ClearScrollback\x07");

    out.push_str(Player::P1.to_color());
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
                    out.push_str(owner.to_color());
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

        out.push(to_hex_digit(i as u8 + 10)); // + 10 so that it prints A, B, C, D

        for j in row {
            match &game_state.board[j] {
                Cell::Card { owner, card } => {
                    out.push_str(owner.to_color());
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
                    out.push_str(owner.to_color());
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

    out.push_str(Player::P2.to_color());
    push_hand(out, &game_state.p2_hand);
    out.push_str(RESET);

    out.push_str(game_state.turn.to_color());
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

fn push_card_stats(out: &mut String, card: &Card) {
    out.push(to_hex_digit(card.attack));
    out.push(card.card_type.to_char());
    out.push(to_hex_digit(card.physical_defense));
    out.push(to_hex_digit(card.magical_defense));
}

fn push_hand(out: &mut String, hand: &[Option<Card>; 5]) {
    for (i, card) in hand.iter().enumerate() {
        if card.is_some() {
            out.push_str("╔═══ ");
            out.push(to_hex_digit(i as u8 + 1));
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

impl Player {
    fn to_color(self) -> &'static str {
        match self {
            Player::P1 => FG_BLUE,
            Player::P2 => FG_RED,
        }
    }
}

impl CardType {
    fn to_char(self) -> char {
        use CardType::*;
        match self {
            Physical => 'P',
            Magical => 'M',
            Exploit => 'X',
            Assault => 'A',
        }
    }
}

fn to_hex_digit(num: u8) -> char {
    match num {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'A',
        11 => 'B',
        12 => 'C',
        13 => 'D',
        14 => 'E',
        15 => 'F',
        _ => unreachable!(),
    }
}
