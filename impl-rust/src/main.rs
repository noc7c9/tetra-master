mod game_log;
mod input;
mod logic;
mod render;
mod render_simple;
mod rng;

use game_log::{Entry, GameLog};
use rng::{Rng, Seed};

const MAX_NUMBER_OF_BLOCKS: u8 = 6;
const HAND_CANDIDATES: usize = 3;
const HAND_SIZE: usize = 5;
const BOARD_SIZE: usize = 4 * 4;

type Hand = [Option<Card>; HAND_SIZE];
type HandCandidate = [Card; HAND_SIZE];
type HandCandidates = [HandCandidate; HAND_CANDIDATES];
type Board = [Cell; BOARD_SIZE];

#[derive(Debug, Clone, Copy)]
enum BattleSystem {
    Original,
    Dice { sides: u8 },
}

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
    fn new(
        attack: u8,
        card_type: CardType,
        physical_defense: u8,
        magical_defense: u8,
        arrows: Arrows,
    ) -> Self {
        assert!(attack <= 0xF, "attack outside expected range 0-F");
        assert!(
            physical_defense <= 0xF,
            "physical defense outside expected range 0-F"
        );
        assert!(
            magical_defense <= 0xF,
            "magical defense outside expected range 0-F"
        );
        Card {
            card_type,
            attack,
            physical_defense,
            magical_defense,
            arrows,
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct BattleResult {
    winner: BattleWinner,
    attack_stat: BattleStat,
    defense_stat: BattleStat,
}

/*****************************************************************************************
 * PreGame Types
 */

#[derive(Debug, Clone, PartialEq)]
enum PreGameStatus {
    P1Picking,
    P2Picking { p1_pick: usize },
    Complete { p1_pick: usize, p2_pick: usize },
}

#[derive(Debug, Clone)]
struct PreGameState {
    status: PreGameStatus,
    rng: Rng,
    board: Board,
    hand_candidates: HandCandidates,
}

impl PreGameState {
    fn with_rng(rng: Rng) -> Self {
        let status = PreGameStatus::P1Picking;
        let board = rng::random_board(&rng);
        let hand_candidates = rng::random_hand_candidates(&rng);

        Self {
            status,
            rng,
            board,
            hand_candidates,
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.status, PreGameStatus::Complete { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PreGameInput {
    pick: usize,
}

/*****************************************************************************************
 * Game Types
 */

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

#[derive(Debug, Clone)]
struct GameState {
    status: GameStatus,
    rng: Rng,
    turn: Player,
    board: Board,
    p1_hand: Hand,
    p2_hand: Hand,
    battle_system: BattleSystem,
}

impl GameState {
    fn from_pre_game_state(pre_game_state: PreGameState, battle_system: BattleSystem) -> Self {
        fn convert_hand([a, b, c, d, e]: HandCandidate) -> Hand {
            [Some(a), Some(b), Some(c), Some(d), Some(e)]
        }

        let status = GameStatus::WaitingPlace;
        let turn = Player::P1;

        let rng = pre_game_state.rng;
        let board = pre_game_state.board;

        let (p1_pick, p2_pick) = match pre_game_state.status {
            PreGameStatus::Complete { p1_pick, p2_pick } => (p1_pick, p2_pick),
            _ => panic!("Cannot get picks from an incomplete PreGameState"),
        };
        let p1_hand = convert_hand(pre_game_state.hand_candidates[p1_pick]);
        let p2_hand = convert_hand(pre_game_state.hand_candidates[p2_pick]);

        Self {
            status,
            rng,
            turn,
            board,
            p1_hand,
            p2_hand,
            battle_system,
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
        matches!(self.status, GameStatus::GameOver { .. })
    }
}

#[derive(Debug, Clone, Copy)]
enum GameInput {
    Place(GameInputPlace),
    Battle(GameInputBattle),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameInputPlace {
    card: usize,
    cell: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameInputBattle {
    cell: usize,
}

/*****************************************************************************************
 * Args
 */

struct Args {
    battle_system: BattleSystem,
    simple_ui: bool,
    seed: Option<u64>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        battle_system: BattleSystem::Original,
        simple_ui: false,
        seed: None,
    };
    for arg in std::env::args() {
        // --dice
        if let Some((_, sides)) = arg.split_once("--dice=") {
            args.battle_system = if let Ok(sides) = sides.parse() {
                BattleSystem::Dice { sides }
            } else {
                return Err(format!("{sides} isn't a valid dice value"));
            };
        } else if arg == "--dice" {
            args.battle_system = BattleSystem::Dice { sides: 6 }
        }
        // --simple-ui
        else if arg == "--simple-ui" {
            args.simple_ui = true;
        }
        // --seed
        else if let Some((_, seed)) = arg.split_once("--seed=") {
            args.seed = if let Ok(seed) = seed.parse() {
                Some(seed)
            } else {
                return Err(format!("{seed} isn't a valid seed value"));
            };
        }
    }
    Ok(args)
}

/*****************************************************************************************
 * main
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{BufRead, Write};

    let args = parse_args()?;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let stdin = std::io::stdin();
    let mut in_ = stdin.lock();

    let mut buf = String::new();

    let mut log = GameLog::new();

    // pre-game loop
    let mut state = PreGameState::with_rng(args.seed.map_or_else(Rng::new, Rng::with_seed));
    loop {
        buf.clear();
        if args.simple_ui {
            render_simple::pre_game_screen(&mut buf, &state)?;
        } else {
            render::pre_game_screen(&mut buf, &state)?;
        }
        out.write_all(buf.as_bytes())?;
        out.flush()?;

        if state.is_complete() {
            break;
        }

        // input loop
        loop {
            out.write_all(b"> ")?;
            out.flush()?;

            // read and parse input
            buf.clear();
            in_.read_line(&mut buf)?;
            let input = match input::parse_pre_game(&buf) {
                Err(input::Error::EmptyInput) => continue,
                Err(err) => {
                    println!("ERR: {}", err);
                    continue;
                }
                Ok(input) => input,
            };

            if let Err(err) = logic::pre_game_next(&mut state, &mut log, input) {
                println!("ERR: {}", err);
            } else {
                // input was correctly evaluated, break input loop
                break;
            }
        }
    }

    // game loop
    let mut state = GameState::from_pre_game_state(state, args.battle_system);
    loop {
        buf.clear();
        if args.simple_ui {
            render_simple::game_screen(&mut buf, &log, &state)?;
        } else {
            render::game_screen(&mut buf, &log, &state)?;
        }
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
            let input = match input::parse_game(&state, &buf) {
                Err(input::Error::EmptyInput) => continue,
                Err(err) => {
                    println!("ERR: {}", err);
                    continue;
                }
                Ok(input) => input,
            };

            if let Err(err) = logic::game_next(&mut state, &mut log, input) {
                println!("ERR: {}", err);
            } else {
                // input was correctly evaluated, break input loop
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[test]
fn type_sizes() {
    use std::mem::size_of;

    let max = size_of::<u64>();

    assert!(size_of::<Arrows>() < max);
    assert!(size_of::<BattleResult>() < max);
    assert!(size_of::<BattleStat>() < max);
    assert!(size_of::<BattleSystem>() < max);
    assert!(size_of::<BattleWinner>() < max);
    assert!(size_of::<Card>() < max);
    assert!(size_of::<CardType>() < max);
    assert!(size_of::<Cell>() < max);
    assert!(size_of::<OwnedCard>() < max);
    assert!(size_of::<Player>() < max);
}
