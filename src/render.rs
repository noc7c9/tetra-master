use crate::{
    game_log::{self, GameLog},
    Card, CardType, Cell, GameState, Player,
};

const RED: &str = "\x1b[0;31m";
const BLUE: &str = "\x1b[0;34m";
const GRAY: &str = "\x1b[0;30m";
const GRAY_BOLD: &str = "\x1b[1;30m";
const RESET: &str = "\x1b[0m";

pub(crate) fn screen(game_log: &GameLog, game_state: &GameState, out: &mut String) {
    // clear screen first thing
    out.push_str("\x1b]50;ClearScrollback\x07");

    out.push_str(Player::P1.to_color());
    push_hand(out, &game_state.p1_hand);
    out.push_str(RESET);

    out.push_str("\n   ┌───────────┬───────────┬───────────┬───────────┐\n");

    for (idx, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
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
                    out.push_str(GRAY_BOLD);
                    out.push_str(" ╔═══════╗ ");
                    out.push_str(RESET);
                }
                Cell::Empty => out.push_str("           "),
            }
            out.push('│');
        }

        out.push_str("\n   │");
        for j in row {
            if let Cell::Blocked = &game_state.board[j] {
                out.push_str(GRAY_BOLD);
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
                    out.push(if card.arrows.left { '⇐' } else { ' ' });
                    out.push_str("  ");
                    push_card_stats(out, card);
                    out.push(' ');
                    out.push(if card.arrows.right { '⇒' } else { ' ' });
                    out.push(' ');
                    out.push_str(RESET);
                }
                Cell::Blocked => {
                    out.push_str(GRAY_BOLD);
                    out.push_str(" ║ BLOCK ║ ");
                    out.push_str(RESET);
                }
                Cell::Empty => {
                    out.push_str("     ");
                    out.push(to_hex_digit(j as u8));
                    out.push_str("     ");
                }
            }
            out.push('│');
        }

        out.push_str("\n   │");
        for j in row {
            if let Cell::Blocked = &game_state.board[j] {
                out.push_str(GRAY_BOLD);
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
                    out.push_str(GRAY_BOLD);
                    out.push_str(" ╚═══════╝ ");
                    out.push_str(RESET);
                }
                Cell::Empty => out.push_str("           "),
            }
            out.push('│');
        }

        if idx != 3 {
            out.push_str("\n   ├───────────┼───────────┼───────────┼───────────┤\n");
        }
    }

    out.push_str("\n   └───────────┴───────────┴───────────┴───────────┘\n\n");

    out.push_str(Player::P2.to_color());
    push_hand(out, &game_state.p2_hand);
    out.push_str(RESET);

    // render game log
    out.push_str(GRAY_BOLD);
    out.push_str("\n                     ══ GAMELOG ══\n");
    out.push_str(RESET);
    let mut prev_turn_number = 0;
    for entry in game_log.iter() {
        use std::fmt::Write;
        out.push_str(entry.turn.to_color());
        if entry.turn_number == prev_turn_number {
            out.push_str("         ");
        } else if entry.turn_number < 10 {
            write!(out, "  Turn {} ", entry.turn_number).unwrap();
        } else {
            out.push_str(" Turn 10 ");
        }
        prev_turn_number = entry.turn_number;
        out.push_str(RESET);
        out.push_str("│ ");
        match &entry.data {
            game_log::EntryData::PlaceCard { card, cell } => {
                out.push_str("Placed  ");
                out.push_str(entry.turn.to_color());
                push_card_stats(out, card);
                out.push_str(RESET);
                out.push_str(" on cell ");
                out.push(to_hex_digit(*cell as u8));
            }
            game_log::EntryData::FlipCard { card, cell, to } => {
                out.push_str("Flipped ");
                out.push_str(to.opposite().to_color());
                push_card_stats(out, card);
                out.push_str(RESET);
                out.push_str(" on cell ");
                out.push(to_hex_digit(*cell as u8));
                out.push_str(" to ");
                out.push_str(to.to_color());
                out.push_str(to.to_color_name());
                out.push_str(RESET);
            }
        }
        out.push('\n');
    }

    // render prompt
    out.push_str("\nTurn: ");
    out.push_str(game_state.turn.to_color());
    if game_state.turn == Player::P1 {
        out.push_str("Player 1");
    } else {
        out.push_str("Player 2");
    }

    out.push_str(GRAY);
    out.push_str(" [ format: {CARD#} {COORD} ]\n");
    out.push_str(RESET);
}

fn push_card_stats(out: &mut String, card: &Card) {
    out.push(to_hex_digit(card.attack));
    out.push(card.card_type.to_char());
    out.push(to_hex_digit(card.physical_defense));
    out.push(to_hex_digit(card.magical_defense));
}

fn push_hand(out: &mut String, hand: &[Option<Card>; 5]) {
    for (idx, card) in hand.iter().enumerate() {
        if card.is_some() {
            out.push_str("╔═══ ");
            out.push(to_hex_digit(idx as u8));
            out.push_str(" ═══╗");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if let Some(card) = card {
            out.push_str("║ ");
            out.push(if card.arrows.top_left { '⇖' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.top { '⇑' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.top_right { '⇗' } else { ' ' });
            out.push_str(" ║");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if let Some(card) = card {
            out.push_str("║ ");
            out.push(if card.arrows.left { '⇐' } else { ' ' });
            out.push(' ');
            push_card_stats(out, card);
            out.push(if card.arrows.right { '⇒' } else { ' ' });
            out.push_str(" ║");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if let Some(card) = card {
            out.push_str("║ ");
            out.push(if card.arrows.bottom_left { '⇙' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.bottom { '⇓' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.bottom_right { '⇘' } else { ' ' });
            out.push_str(" ║");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if card.is_some() {
            out.push_str("╚═════════╝");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');
}

impl Player {
    fn to_color(self) -> &'static str {
        match self {
            Player::P1 => BLUE,
            Player::P2 => RED,
        }
    }

    fn to_color_name(self) -> &'static str {
        match self {
            Player::P1 => "blue",
            Player::P2 => "red",
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
