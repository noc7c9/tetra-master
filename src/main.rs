const COLOR_RED: &str = "\x1b[0;31m";
const COLOR_BLUE: &str = "\x1b[0;34m";
const COLOR_RESET: &str = "\x1b[0m";

struct GameState {
    board: [Option<Card>; 4 * 4],
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

#[derive(Clone)]
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
    owner: Player,
    card_type: CardType,
    attack: u8,
    physical_defense: u8,
    magical_defense: u8,
    arrows: Arrows,
}

impl Card {
    fn new(owner: Player, stats: &str, arrows: Arrows) -> Self {
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
            owner,
            card_type,
            attack,
            physical_defense,
            magical_defense,
            arrows,
        }
    }
}

fn render_screen(game_state: &GameState, out: &mut String) {
    fn push_hex_digit(num: u8, out: &mut String) {
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

    fn push_card_type(card_type: CardType, out: &mut String) {
        use CardType::*;
        match card_type {
            Physical => out.push('P'),
            Magical => out.push('M'),
            Exploit => out.push('X'),
            Assault => out.push('A'),
        }
    }

    fn push_card_color(card: &Option<Card>, out: &mut String) {
        if let Some(card) = card {
            out.push_str(if card.owner == Player::P1 {
                COLOR_BLUE
            } else {
                COLOR_RED
            });
        }
    }

    fn push_card_line_1(card: &Option<Card>, out: &mut String) {
        push_card_color(card, out);
        if let Some(card) = card {
            out.push(if card.arrows.top_left { '⇖' } else { ' ' });
            out.push_str("   ");
            out.push(if card.arrows.top { '⇑' } else { ' ' });
            out.push_str("   ");
            out.push(if card.arrows.top_right { '⇗' } else { ' ' });
        } else {
            out.push_str("         ");
        }
        out.push_str(COLOR_RESET);
    }
    fn push_card_line_2(card: &Option<Card>, out: &mut String) {
        push_card_color(card, out);
        if let Some(card) = card {
            out.push(if card.arrows.left { '⇐' } else { ' ' });
            out.push_str("  ");
            push_hex_digit(card.attack, out);
            push_card_type(card.card_type, out);
            push_hex_digit(card.physical_defense, out);
            push_hex_digit(card.magical_defense, out);
            out.push(' ');
            out.push(if card.arrows.right { '⇒' } else { ' ' });
        } else {
            out.push_str("         ");
        }
        out.push_str(COLOR_RESET);
    }
    fn push_card_line_3(card: &Option<Card>, out: &mut String) {
        push_card_color(card, out);
        if let Some(card) = card {
            out.push(if card.arrows.bottom_left { '⇙' } else { ' ' });
            out.push_str("   ");
            out.push(if card.arrows.bottom { '⇓' } else { ' ' });
            out.push_str("   ");
            out.push(if card.arrows.bottom_right { '⇘' } else { ' ' });
        } else {
            out.push_str("         ");
        }
        out.push_str(COLOR_RESET);
    }

    out.push_str("┌───────────┬───────────┬───────────┬───────────┐\n");

    for (i, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        out.push_str("│ ");
        for j in row {
            push_card_line_1(&game_state.board[j], out);
            out.push_str(" │ ");
        }

        out.push_str("\n│           │           │           │           │\n");

        out.push_str("│ ");
        for j in row {
            push_card_line_2(&game_state.board[j], out);
            out.push_str(" │ ");
        }

        out.push_str("\n│           │           │           │           │\n");

        out.push_str("│ ");
        for j in row {
            push_card_line_3(&game_state.board[j], out);
            out.push_str(" │ ");
        }

        if i != 3 {
            out.push_str("\n├───────────┼───────────┼───────────┼───────────┤\n");
        }
    }

    out.push_str("\n└───────────┴───────────┴───────────┴───────────┘\n");
}

fn clear_screen(out: &mut String) {
    out.push_str("\x1b]50;ClearScrollback\x07")
}

fn main() {
    let card1 = Card::new(
        Player::P1,
        "9P2F",
        Arrows {
            top_left: true,
            top: false,
            top_right: true,
            left: true,
            right: false,
            bottom_left: false,
            bottom: true,
            bottom_right: true,
        },
    );
    let card2 = Card {
        owner: Player::P2,
        ..card1.clone()
    };
    let mut state = GameState {
        board: [
            Some(card1),
            None,
            Some(card2),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
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
