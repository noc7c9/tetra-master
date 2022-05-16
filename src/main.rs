mod game_log;
mod input;
mod logic;
mod render;

pub(crate) use game_log::{Entry, GameLog};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Player {
    P1,
    P2,
}

impl Player {
    fn opposite(self) -> Self {
        match self {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CardType {
    Physical,
    Magical,
    Exploit,
    Assault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct Card {
    card_type: CardType,
    attack: u8,
    physical_defense: u8,
    magical_defense: u8,
    arrows: Arrows,
}

impl Card {
    fn random(rng: &fastrand::Rng) -> Self {
        fn randpick<'a, T>(rng: &fastrand::Rng, values: &'a [T]) -> &'a T {
            let len = values.len();
            let idx = rng.usize(..len);
            &values[idx]
        }

        fn random_stat(rng: &fastrand::Rng) -> u8 {
            *match rng.f32() {
                n if n < 0.05 => randpick(rng, &[0, 1]),
                n if n < 0.35 => randpick(rng, &[2, 3, 4, 5]),
                n if n < 0.8 => randpick(rng, &[6, 7, 8, 9, 10]),
                n if n < 0.95 => randpick(rng, &[11, 12, 13]),
                _ => randpick(rng, &[14, 15]),
            }
        }

        let card_type = match rng.f32() {
            n if n < 0.40 => CardType::Physical, // 40%
            n if n < 0.80 => CardType::Magical,  // 40%
            n if n < 0.95 => CardType::Exploit,  // 15%
            _ => CardType::Assault,              // 5%
        };

        let arrows = Arrows {
            top_left: rng.bool(),
            top: rng.bool(),
            top_right: rng.bool(),
            left: rng.bool(),
            right: rng.bool(),
            bottom_left: rng.bool(),
            bottom: rng.bool(),
            bottom_right: rng.bool(),
        };

        Card {
            card_type,
            arrows,
            attack: random_stat(rng),
            physical_defense: random_stat(rng),
            magical_defense: random_stat(rng),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OwnedCard {
    owner: Player,
    card: Card,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Cell {
    Blocked,
    Card(OwnedCard),
    Empty,
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BattleWinner {
    Attacker,
    Defender,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BattleStat {
    digit: u8,
    value: u8,
    roll: u8,
}

impl BattleStat {
    fn resolve(self) -> u8 {
        self.value - self.roll
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BattleResult {
    winner: BattleWinner,
    attack_stat: BattleStat,
    defense_stat: BattleStat,
}

#[derive(Debug, Clone)]
struct GameState {
    rng: fastrand::Rng,
    turn: Player,
    board: [Cell; 4 * 4],
    p1_hand: [Option<Card>; 5],
    p2_hand: [Option<Card>; 5],
}

impl GameState {
    fn with_seed(seed: u64) -> Self {
        let rng = fastrand::Rng::with_seed(seed);
        let turn = if rng.bool() { Player::P1 } else { Player::P2 };
        let mut board: [Cell; 4 * 4] = Default::default();
        let p1_hand: [Option<Card>; 5] = [
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
        ];
        let p2_hand: [Option<Card>; 5] = [
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
        ];

        // block 0-6 cells
        for _ in 0..=rng.u8(..=6) {
            let idx = rng.usize(..(4 * 4));
            board[idx] = Cell::Blocked;
        }

        GameState {
            rng,
            turn,
            board,
            p1_hand,
            p2_hand,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Move {
    card: usize,
    cell: usize,
}

fn main() {
    let mut state = GameState::with_seed(fastrand::u64(..));
    let mut log = GameLog::new(state.turn);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let stdin = std::io::stdin();
    let mut in_ = stdin.lock();

    let mut buf = String::new();
    loop {
        use std::io::{BufRead, Write};

        buf.clear();
        render::clear(&mut buf);
        render::screen(&log, &state, &mut buf);
        out.write_all(buf.as_bytes()).unwrap();
        out.flush().unwrap();

        loop {
            out.write_all(b"> ").unwrap();
            out.flush().unwrap();

            buf.clear();
            in_.read_line(&mut buf).unwrap();
            match input::parse(&buf).and_then(|input| logic::next(&mut state, &mut log, input)) {
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
