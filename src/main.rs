mod input;
mod logic;
mod render;

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
    fn random_card_type() -> CardType {
        match fastrand::f32() {
            n if n < 0.40 => CardType::Physical, // 40%
            n if n < 0.80 => CardType::Magical,  // 40%
            n if n < 0.95 => CardType::Exploit,  // 15%
            _ => CardType::Assault,              // 5%
        }
    }

    fn random_stat() -> u8 {
        fn randpick<T>(values: &[T]) -> &T {
            let len = values.len();
            let idx = fastrand::usize(..len);
            &values[idx]
        }

        *match fastrand::f32() {
            n if n < 0.05 => randpick(&[0, 1]),
            n if n < 0.35 => randpick(&[2, 3, 4, 5]),
            n if n < 0.8 => randpick(&[6, 7, 8, 9, 10]),
            n if n < 0.95 => randpick(&[11, 12, 13]),
            _ => randpick(&[14, 15]),
        }
    }

    fn random_arrows() -> Arrows {
        Arrows {
            top_left: fastrand::bool(),
            top: fastrand::bool(),
            top_right: fastrand::bool(),
            left: fastrand::bool(),
            right: fastrand::bool(),
            bottom_left: fastrand::bool(),
            bottom: fastrand::bool(),
            bottom_right: fastrand::bool(),
        }
    }

    fn new_random() -> Self {
        Card {
            card_type: Self::random_card_type(),
            attack: Self::random_stat(),
            physical_defense: Self::random_stat(),
            magical_defense: Self::random_stat(),
            arrows: Self::random_arrows(),
        }
    }
}

enum Cell {
    Blocked,
    Card { owner: Player, card: Card },
    Empty,
}

impl Cell {
    fn is_empty(&self) -> bool {
        matches!(self, Cell::Empty)
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

struct GameState {
    turn: Player,
    board: [Cell; 4 * 4],
    p1_hand: [Option<Card>; 5],
    p2_hand: [Option<Card>; 5],
}

impl GameState {
    fn new() -> Self {
        let turn = Player::P1;
        let mut board: [Cell; 4 * 4] = Default::default();
        let p1_hand: [Option<Card>; 5] = [
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
        ];
        let p2_hand: [Option<Card>; 5] = [
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
            Some(Card::new_random()),
        ];

        // block 0-6 cells
        for _ in 0..=fastrand::u8(..=6) {
            let idx = fastrand::usize(..(4 * 4));
            board[idx] = Cell::Blocked;
        }

        GameState {
            turn,
            board,
            p1_hand,
            p2_hand,
        }
    }
}

struct Move {
    card: usize,
    cell: usize,
}

enum GameLogEntry {
    PlaceCard { card: Card, cell: usize },
}

struct GameLog(Vec<(usize, Player, GameLogEntry)>);

impl GameLog {
    fn new() -> Self {
        GameLog(Vec::new())
    }

    fn append(&mut self, player: Player, entry: GameLogEntry) {
        self.0.push((self.0.len() + 1, player, entry))
    }

    fn iter(&self) -> std::slice::Iter<'_, (usize, Player, GameLogEntry)> {
        self.0.iter()
    }
}

fn main() {
    let mut game_log = GameLog::new();
    let mut game_state = GameState::new();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let stdin = std::io::stdin();
    let mut in_ = stdin.lock();

    let mut buf = String::new();
    loop {
        use std::io::{BufRead, Write};

        buf.clear();
        render::screen(&game_log, &game_state, &mut buf);
        out.write_all(buf.as_bytes()).unwrap();
        out.flush().unwrap();

        loop {
            out.write_all(b"> ").unwrap();
            out.flush().unwrap();

            buf.clear();
            in_.read_line(&mut buf).unwrap();
            match input::parse(&buf)
                .and_then(|input| logic::next(&mut game_state, &mut game_log, input))
            {
                Ok(_) => {
                    break;
                }
                Err(err) => {
                    println!("ERR: {}", err);
                }
            }
        }
    }
}
