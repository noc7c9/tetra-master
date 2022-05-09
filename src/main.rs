mod input;
mod render;

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
        render::screen(&game_state, &mut buf);
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
            match input::parse(&buf) {
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
