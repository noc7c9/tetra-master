mod game_log;
mod input;
mod logic;
mod render;

const HAND_SIZE: usize = 5;
const BOARD_SIZE: usize = 4 * 4;
const MAX_NUMBER_OF_BLOCKS: u8 = 6;

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

// use a bitset to make the type smaller
#[derive(Debug, Clone, Copy, PartialEq)]
struct Arrows(u8);

impl Arrows {
    #[cfg(test)]
    const NONE: Arrows = Arrows(0b0000_0000);

    #[cfg(test)]
    const ALL: Arrows = Arrows(0b1111_1111);

    // clockwise from the top
    const UP: Arrows = Arrows(0b1000_0000);
    const UP_RIGHT: Arrows = Arrows(0b0100_0000);
    const RIGHT: Arrows = Arrows(0b0010_0000);
    const DOWN_RIGHT: Arrows = Arrows(0b0001_0000);
    const DOWN: Arrows = Arrows(0b0000_1000);
    const DOWN_LEFT: Arrows = Arrows(0b0000_0100);
    const LEFT: Arrows = Arrows(0b0000_0010);
    const UP_LEFT: Arrows = Arrows(0b0000_0001);

    // returns an Arrows with all of the arrows pointing in the opposite direction
    fn reverse(self) -> Self {
        // wrapping shift by 4 bits
        // this is effectively rotating the arrows by 180 degrees
        Arrows(self.0 >> 4 | self.0 << 4)
    }

    fn has(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    fn up_left(self) -> bool {
        self.has(Arrows::UP_LEFT)
    }

    fn up(self) -> bool {
        self.has(Arrows::UP)
    }

    fn up_right(self) -> bool {
        self.has(Arrows::UP_RIGHT)
    }

    fn left(self) -> bool {
        self.has(Arrows::LEFT)
    }

    fn right(self) -> bool {
        self.has(Arrows::RIGHT)
    }

    fn down_left(self) -> bool {
        self.has(Arrows::DOWN_LEFT)
    }

    fn down(self) -> bool {
        self.has(Arrows::DOWN)
    }

    fn down_right(self) -> bool {
        self.has(Arrows::DOWN_RIGHT)
    }
}

impl std::ops::BitOr for Arrows {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Arrows(self.0 | rhs.0)
    }
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
            let base_stat = *match rng.f32() {
                n if n < 0.05 => randpick(rng, &[0, 1]),          // 5%
                n if n < 0.35 => randpick(rng, &[2, 3, 4, 5]),    // 30%
                n if n < 0.8 => randpick(rng, &[6, 7, 8, 9, 10]), // 45%
                n if n < 0.95 => randpick(rng, &[11, 12, 13]),    // 15%
                _ => randpick(rng, &[14, 15]),                    // 5%
            };
            // base stats range from 0x0 to 0xF
            // real stats range from 0x0 to 0xFF
            0x10 * base_stat + rng.u8(..16)
        }

        let card_type = match rng.f32() {
            n if n < 0.40 => CardType::Physical, // 40%
            n if n < 0.80 => CardType::Magical,  // 40%
            n if n < 0.95 => CardType::Exploit,  // 15%
            _ => CardType::Assault,              // 5%
        };

        let arrows = Arrows(rng.u8(..));

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

#[derive(Debug, Clone, PartialEq)]
enum GameStatus {
    WaitingPlace,
    WaitingBattle {
        attacker_cell: usize,
        choices: Vec<(usize, Card)>,
    },
    GameOver {
        winner: Option<Player>,
    },
}

type Hand = [Option<Card>; HAND_SIZE];
type Board = [Cell; BOARD_SIZE];

#[derive(Debug, Clone)]
struct GameState {
    status: GameStatus,
    rng: fastrand::Rng,
    turn: Player,
    board: Board,
    p1_hand: Hand,
    p2_hand: Hand,
}

impl GameState {
    fn with_seed(seed: u64) -> Self {
        let status = GameStatus::WaitingPlace;
        let rng = fastrand::Rng::with_seed(seed);
        let turn = Player::P1;
        let mut board = Board::default();
        let p1_hand: Hand = [
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
        ];
        let p2_hand: Hand = [
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
            Some(Card::random(&rng)),
        ];

        // block cells
        for _ in 0..rng.u8(..=MAX_NUMBER_OF_BLOCKS) {
            let idx = rng.usize(..(HAND_SIZE));
            board[idx] = Cell::Blocked;
        }

        GameState {
            status,
            rng,
            turn,
            board,
            p1_hand,
            p2_hand,
        }
    }

    // take out card from the given cell
    // panics if there is no card in the given cell
    fn take_card(&mut self, cell: usize) -> OwnedCard {
        match std::mem::take(&mut self.board[cell]) {
            Cell::Card(card) => card,
            _ => panic!("Cell didn't have a card"),
        }
    }

    fn is_game_over(&self) -> bool {
        if let GameStatus::GameOver { .. } = self.status {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Copy)]
enum Input {
    Place(InputPlace),
    Battle(InputBattle),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InputPlace {
    card: usize,
    cell: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InputBattle {
    cell: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = GameState::with_seed(fastrand::u64(..));
    let mut log = GameLog::new();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let stdin = std::io::stdin();
    let mut in_ = stdin.lock();

    let mut buf = String::new();
    // game loop
    loop {
        use std::io::{BufRead, Write};

        buf.clear();
        render::screen(&log, &state, &mut buf)?;
        out.write_all(buf.as_bytes())?;
        out.flush()?;

        if state.is_game_over() {
            break;
        }

        // input loop
        loop {
            out.write_all(b"> ")?;
            out.flush()?;

            // read and parse input
            buf.clear();
            in_.read_line(&mut buf)?;
            let input = match input::parse(&state, &buf) {
                Err(input::Error::EmptyInput) => continue,
                Err(err) => {
                    println!("ERR: {}", err);
                    continue;
                }
                Ok(input) => input,
            };

            if let Err(err) = logic::next(&mut state, &mut log, input) {
                println!("ERR: {}", err);
            } else {
                // input was correctly evaluated, break input loop
                break;
            }
        }
    }

    Ok(())
}
