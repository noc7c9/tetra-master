use crate::{
    BattleWinner, Card, CardType, Cell, Entry, GameLog, GameState, GameStatus, OwnedCard, Player,
};

const RED: &str = "\x1b[0;31m";
const RED_BOLD: &str = "\x1b[1;31m";
const BLUE: &str = "\x1b[0;34m";
const BLUE_BOLD: &str = "\x1b[1;34m";
const GRAY: &str = "\x1b[0;30m";
const GRAY_BOLD: &str = "\x1b[1;30m";
const WHITE_BOLD: &str = "\x1b[1;37m";
const RESET: &str = "\x1b[0m";

pub(crate) fn clear(out: &mut String) {
    out.push_str("\x1b]50;ClearScrollback\x07");
}

pub(crate) fn screen(log: &GameLog, state: &GameState, out: &mut String) {
    push_hand(out, Player::P1, &state.p1_hand);
    out.push('\n');

    push_board(out, state);
    out.push('\n');

    push_hand(out, Player::P2, &state.p2_hand);
    out.push('\n');

    push_game_log(out, log);

    if let GameStatus::GameOver { winner } = state.status {
        push_game_over(out, winner);
    } else {
        push_prompt(out, state);
    }
}

fn push_hand(out: &mut String, owner: Player, hand: &[Option<Card>; 5]) {
    out.push_str(owner.to_color());

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
            out.push(if card.arrows.up_left() { '⇖' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.up() { '⇑' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.up_right() { '⇗' } else { ' ' });
            out.push_str(" ║");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if let Some(card) = card {
            out.push_str("║ ");
            out.push(if card.arrows.left() { '⇐' } else { ' ' });
            out.push(' ');
            push_card_stats(out, *card);
            out.push(if card.arrows.right() { '⇒' } else { ' ' });
            out.push_str(" ║");
        } else {
            out.push_str("           ");
        }
    }
    out.push('\n');

    for card in hand {
        if let Some(card) = card {
            out.push_str("║ ");
            out.push(if card.arrows.down_left() { '⇙' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.down() { '⇓' } else { ' ' });
            out.push_str("  ");
            out.push(if card.arrows.down_right() { '⇘' } else { ' ' });
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

    out.push_str(RESET);
}

fn push_board(out: &mut String, state: &GameState) {
    out.push_str("   ┌───────────┬───────────┬───────────┬───────────┐\n");

    for (idx, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        out.push_str("   │");
        for j in row {
            match &state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    out.push_str(owner.to_color());
                    out.push(' ');
                    out.push(if card.arrows.up_left() { '⇖' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.up() { '⇑' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.up_right() { '⇗' } else { ' ' });
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
            if let Cell::Blocked = &state.board[j] {
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
            match state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    out.push_str(owner.to_color());
                    out.push(' ');
                    out.push(if card.arrows.left() { '⇐' } else { ' ' });
                    out.push_str("  ");
                    push_card_stats(out, card);
                    out.push(' ');
                    out.push(if card.arrows.right() { '⇒' } else { ' ' });
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
            if let Cell::Blocked = &state.board[j] {
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
            match &state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    out.push_str(owner.to_color());
                    out.push(' ');
                    out.push(if card.arrows.down_left() { '⇙' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.down() { '⇓' } else { ' ' });
                    out.push_str("   ");
                    out.push(if card.arrows.down_right() { '⇘' } else { ' ' });
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

    out.push_str("\n   └───────────┴───────────┴───────────┴───────────┘\n");
}

fn push_game_log(out: &mut String, log: &GameLog) {
    out.push_str(GRAY_BOLD);
    out.push_str("                     ══ GAMELOG ══\n");
    out.push_str(RESET);

    let mut curr_turn_number = 0;
    let mut curr_turn = Player::P1; // note: initial value will be overwritten immediately
    let mut print_prefix = true;
    for entry in log.iter() {
        if let Entry::NextTurn { turn } = entry {
            curr_turn_number += 1;
            curr_turn = *turn;
            print_prefix = true;
            continue;
        }

        out.push_str(curr_turn.to_color());
        if !print_prefix {
            out.push_str("           ");
        } else if curr_turn_number < 10 {
            use std::fmt::Write;
            write!(out, "    Turn {curr_turn_number} ").unwrap();
        } else {
            out.push_str("   Turn 10 ");
        }
        print_prefix = false;
        out.push_str(RESET);
        out.push_str("│ ");

        match entry {
            Entry::PlaceCard { card, cell } => {
                out.push_str("Placed  ");
                out.push_str(card.owner.to_color());
                push_card_stats(out, card.card);
                out.push_str(RESET);
                out.push_str(" on cell ");
                out.push(to_hex_digit(*cell as u8));
            }

            Entry::FlipCard {
                card,
                cell,
                to,
                via_combo,
            } => {
                if *via_combo {
                    out.push_str("Combo'd ");
                } else {
                    out.push_str("Flipped ");
                }
                out.push_str(to.opposite().to_color());
                push_card_stats(out, card.card);
                out.push_str(RESET);
                out.push_str(" on cell ");
                out.push(to_hex_digit(*cell as u8));
                out.push_str(" to ");
                out.push_str(to.to_color());
                out.push_str(to.to_color_name());
                out.push_str(RESET);
            }

            Entry::Battle {
                attacker,
                defender,
                result,
            } => {
                use std::fmt::Write;

                out.push_str("Battle  ");
                push_card_stats_with_highlight(
                    out,
                    attacker.card,
                    attacker.owner,
                    result.attack_stat.digit,
                );
                out.push_str(" vs ");
                out.push_str(defender.owner.to_color());
                push_card_stats_with_highlight(
                    out,
                    defender.card,
                    defender.owner,
                    result.defense_stat.digit,
                );
                out.push_str(RESET);
                out.push_str("\n           │         ");
                out.push_str(attacker.owner.to_color());
                out.push_str("Attacker");
                out.push_str(RESET);
                out.push_str(" (");
                write!(out, "{}", result.attack_stat.value).unwrap();
                out.push_str(") rolled ");
                write!(out, "{}", result.attack_stat.roll).unwrap();
                out.push_str(", ");
                out.push_str(defender.owner.to_color());
                out.push_str("Defender");
                out.push_str(RESET);
                out.push_str(" (");
                write!(out, "{}", result.defense_stat.value).unwrap();
                out.push_str(") rolled ");
                write!(out, "{}", result.defense_stat.roll).unwrap();
                match result.winner {
                    BattleWinner::Attacker => {
                        out.push_str("\n           │         ");
                        out.push_str(attacker.owner.to_color());
                        out.push_str("Attacker wins");
                        out.push_str(RESET);
                        out.push_str(" (");
                        write!(out, "{}", result.attack_stat.resolve()).unwrap();
                        out.push_str(" > ");
                        write!(out, "{}", result.defense_stat.resolve()).unwrap();
                        out.push(')');
                    }
                    BattleWinner::Defender => {
                        out.push_str("\n           │         ");
                        out.push_str(defender.owner.to_color());
                        out.push_str("Defender wins");
                        out.push_str(RESET);
                        out.push_str(" (");
                        write!(out, "{}", result.attack_stat.resolve()).unwrap();
                        out.push_str(" < ");
                        write!(out, "{}", result.defense_stat.resolve()).unwrap();
                        out.push(')');
                    }
                    BattleWinner::None => {
                        out.push_str("\n           │         Draw, ");
                        out.push_str(defender.owner.to_color());
                        out.push_str("defender wins");
                        out.push_str(RESET);
                        out.push_str(" by default (");
                        write!(out, "{}", result.attack_stat.resolve()).unwrap();
                        out.push_str(" = ");
                        write!(out, "{}", result.defense_stat.resolve()).unwrap();
                        out.push(')');
                    }
                }
            }

            Entry::NextTurn { .. } => unreachable!(),
        }
        out.push('\n');
    }
}

fn push_prompt(out: &mut String, state: &GameState) {
    match state.turn {
        Player::P1 => out.push_str("Next: "),
        Player::P2 => out.push_str(" Next: "),
    }
    out.push_str(state.turn.to_color());
    out.push_str(state.turn.to_color_name());
    out.push_str(RESET);
    out.push_str(" │ ");

    match &state.status {
        GameStatus::WaitingPlace => {
            out.push_str("Where to place which card?");
            out.push_str(GRAY);
            out.push_str(" ( format: {CARD#} {COORD} )\n");
            out.push_str(RESET);
        }
        GameStatus::WaitingBattle { choices, .. } => {
            out.push_str(RESET);
            out.push_str("Which card to battle?");
            out.push_str(GRAY);
            out.push_str(" ( format: {COORD} )\n");
            out.push_str(RESET);
            for &(cell, card) in choices {
                out.push_str("  ");
                out.push(to_hex_digit(cell as u8));
                out.push_str(" ( ");
                out.push_str(state.turn.opposite().to_color());
                push_card_stats(out, card);
                out.push_str(RESET);
                out.push_str(" )\n");
            }
        }
        GameStatus::GameOver { .. } => unreachable!(),
    }
}

fn push_game_over(out: &mut String, winner: Option<Player>) {
    out.push(' ');
    out.push_str(WHITE_BOLD);
    out.push_str("Game Over");
    out.push_str(RESET);
    out.push_str(" │ ");
    match winner {
        Some(winner) => {
            out.push_str(winner.to_color());
            out.push_str(winner.to_color_name());
            out.push_str(RESET);
            out.push_str(" Wins\n");
        }
        None => {
            out.push_str("It was a draw!\n");
        }
    }
}

fn push_card_stats(out: &mut String, card: Card) {
    out.push(to_hex_digit(card.attack >> 4));
    out.push(card.card_type.to_char());
    out.push(to_hex_digit(card.physical_defense >> 4));
    out.push(to_hex_digit(card.magical_defense >> 4));
}

fn push_card_stats_with_highlight(out: &mut String, card: Card, owner: Player, highlight: u8) {
    out.push_str(if highlight == 0 {
        owner.to_color_bold()
    } else {
        owner.to_color()
    });
    out.push(to_hex_digit(card.attack >> 4));

    out.push_str(owner.to_color());
    out.push(card.card_type.to_char());

    out.push_str(if highlight == 2 {
        owner.to_color_bold()
    } else {
        owner.to_color()
    });
    out.push(to_hex_digit(card.physical_defense >> 4));

    out.push_str(if highlight == 3 {
        owner.to_color_bold()
    } else {
        owner.to_color()
    });
    out.push(to_hex_digit(card.magical_defense >> 4));

    out.push_str(RESET);
}

impl Player {
    fn to_color(self) -> &'static str {
        match self {
            Player::P1 => BLUE,
            Player::P2 => RED,
        }
    }

    fn to_color_bold(self) -> &'static str {
        match self {
            Player::P1 => BLUE_BOLD,
            Player::P2 => RED_BOLD,
        }
    }

    fn to_color_name(self) -> &'static str {
        match self {
            Player::P1 => "Blue",
            Player::P2 => "Red",
        }
    }
}

impl CardType {
    fn to_char(self) -> char {
        match self {
            CardType::Physical => 'P',
            CardType::Magical => 'M',
            CardType::Exploit => 'X',
            CardType::Assault => 'A',
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
