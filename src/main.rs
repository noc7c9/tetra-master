const COLOR_RED: &str = "\x1b[0;31m";
const COLOR_BLUE: &str = "\x1b[0;34m";
const COLOR_RESET: &str = "\x1b[0m";

struct GameState {
    board: [PlacedCard; 4 * 4],
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

enum PlacedCard {
    Some { owner: Player, card: Card },
    None,
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

    fn push_color(out: &mut String, player: Player) {
        out.push_str(if player == Player::P1 {
            COLOR_BLUE
        } else {
            COLOR_RED
        });
    }
    fn push_reset_color(out: &mut String) {
        out.push_str(COLOR_RESET);
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

    push_color(out, Player::P1);
    push_hand(out, &game_state.p1_hand);
    push_reset_color(out);

    out.push_str("\n   ┌───  1  ───┬───  2  ───┬───  3  ───┬───  4  ───┐\n");

    for (i, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        out.push_str("   │ ");
        for j in row {
            if let PlacedCard::Some { owner, card } = &game_state.board[j] {
                push_color(out, *owner);
                out.push(if card.arrows.top_left { '⇖' } else { ' ' });
                out.push_str("   ");
                out.push(if card.arrows.top { '⇑' } else { ' ' });
                out.push_str("   ");
                out.push(if card.arrows.top_right { '⇗' } else { ' ' });
                push_reset_color(out);
            } else {
                out.push_str("         ");
            }
            out.push_str(" │ ");
        }

        out.push_str("\n               │           │           │           │\n   ");

        push_hex_digit(out, i as u8 + 10); // + 10 so that it prints A, B, C, D
        out.push(' ');

        for j in row {
            if let PlacedCard::Some { owner, card } = &game_state.board[j] {
                push_color(out, *owner);
                out.push(if card.arrows.left { '⇐' } else { ' ' });
                out.push_str("  ");
                push_card_stats(out, card);
                out.push(' ');
                out.push(if card.arrows.right { '⇒' } else { ' ' });
                push_reset_color(out);
            } else {
                out.push_str("         ");
            }
            out.push_str(" │ ");
        }

        out.push_str("\n               │           │           │           │\n   ");

        out.push_str("│ ");
        for j in row {
            if let PlacedCard::Some { owner, card } = &game_state.board[j] {
                push_color(out, *owner);
                out.push(if card.arrows.bottom_left { '⇙' } else { ' ' });
                out.push_str("   ");
                out.push(if card.arrows.bottom { '⇓' } else { ' ' });
                out.push_str("   ");
                out.push(if card.arrows.bottom_right { '⇘' } else { ' ' });
                push_reset_color(out);
            } else {
                out.push_str("         ");
            }
            out.push_str(" │ ");
        }

        if i != 3 {
            out.push_str("\n   ├───────────┼───────────┼───────────┼───────────┤\n");
        }
    }

    out.push_str("\n   └───────────┴───────────┴───────────┴───────────┘\n\n");

    push_color(out, Player::P2);
    push_hand(out, &game_state.p2_hand);
    push_reset_color(out);
}

fn clear_screen(out: &mut String) {
    out.push_str("\x1b]50;ClearScrollback\x07")
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
    let mut state = GameState {
        board: [
            PlacedCard::Some {
                owner: Player::P1,
                card: card.clone(),
            },
            PlacedCard::None,
            PlacedCard::Some {
                owner: Player::P2,
                card: card.clone(),
            },
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
            PlacedCard::None,
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

    let mut buf = String::new();
    loop {
        use std::io::Write;

        buf.clear();
        clear_screen(&mut buf);
        render_screen(&state, &mut buf);
        out.write_all(buf.as_bytes()).unwrap();
        out.flush().unwrap();

        let old_i = i;
        i = (i + 1) % 16;
        state.board.swap(old_i, i);

        let old_j = j;
        j = (j + 1) % 16;
        state.board.swap(old_j, j);

        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
