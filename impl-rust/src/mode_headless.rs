use crate::{
    input, logic, Arrows, BattleSystem, Card, CardType, Cell, GameLog, GameState, HandCandidate,
    HandCandidates, PreGameInput, PreGameState, Rng, Seed,
};
use std::fmt::Write;

#[derive(Debug)]
enum Error {
    HandCandidatesTooShort,
    HandCandidateTooShort,
    InvalidCardType { input: String },
    InvalidHexNumber(std::num::ParseIntError),
    WriteErr(std::fmt::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            HandCandidatesTooShort => f.write_str("hand candidates list too short"),
            HandCandidateTooShort => f.write_str("hand candidate list too short"),
            InvalidCardType { input } => write!(f, "'{input}' is not a valid card type"),
            InvalidHexNumber(inner) => inner.fmt(f),
            WriteErr(inner) => inner.fmt(f),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(inner: std::num::ParseIntError) -> Self {
        Error::InvalidHexNumber(inner)
    }
}

impl From<std::fmt::Error> for Error {
    fn from(inner: std::fmt::Error) -> Self {
        Error::WriteErr(inner)
    }
}

type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum HeadlessState {
    NotInGame,
    InPreGame(PreGameState, BattleSystem),
    InGame(GameState),
}

pub(crate) fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();
    let mut in_ = std::io::stdin().lock();

    let mut buf = String::new();

    let mut current_state = HeadlessState::NotInGame;
    let mut log = GameLog::new();

    loop {
        use std::io::{BufRead, Write};

        // read next command
        buf.clear();
        in_.read_line(&mut buf)?;
        buf.pop(); // drop the new line

        if buf.is_empty() {
            panic!("Unexpected end of command input")
        }

        let mut cmd = buf.split(' ');
        let cmd_name = cmd.next().unwrap();

        if cmd_name == "quit" {
            return Ok(());
        }

        // handle command
        match current_state {
            HeadlessState::NotInGame => {
                if cmd_name == "setup" {
                    let setup = parse_setup(cmd)?;

                    let rng = setup.seed.map_or_else(Rng::new, Rng::with_seed);
                    let mut state = PreGameState::with_rng(rng);

                    if let Some(blocked_cells) = setup.blocked_cells {
                        state.board = Default::default();
                        for cell in blocked_cells {
                            state.board[cell] = Cell::Blocked;
                        }
                    }

                    if let Some(hand_candidates) = setup.hand_candidates {
                        state.hand_candidates = hand_candidates;
                    }

                    let seed = state.rng.initial_seed();
                    let blocked_cells = state.board.iter().enumerate().filter_map(|(idx, cell)| {
                        if let Cell::Blocked = cell {
                            Some(idx)
                        } else {
                            None
                        }
                    });
                    let hand_candidates = &state.hand_candidates;

                    buf.clear();
                    write_setup_ok(&mut buf, seed, blocked_cells, hand_candidates)?;

                    out.write_all(buf.as_bytes())?;
                    out.flush()?;

                    current_state = HeadlessState::InPreGame(state, BattleSystem::Original);

                    continue;
                }
                panic!("Unexpected command {buf}")
            }
            HeadlessState::InPreGame(mut state, battle_system) => {
                if cmd_name == "pick-hand" {
                    let index = parse_pick_hand(cmd)?;

                    buf.clear();

                    let input = PreGameInput { pick: index };
                    if let Err(err) = logic::pre_game_next(&mut state, &mut log, input) {
                        write_pick_hand_err(&mut buf, &err)?;
                    } else {
                        write_pick_hand_ok(&mut buf)?;
                    }
                    out.write_all(buf.as_bytes())?;
                    out.flush()?;

                    if state.is_complete() {
                        let state = GameState::from_pre_game_state(state, battle_system);
                        current_state = HeadlessState::InGame(state);
                    } else {
                        // restore previous state to keep borrow checker happy
                        current_state = HeadlessState::InPreGame(state, battle_system);
                    }

                    continue;
                }
                panic!("Unexpected command {buf}")
            }
            HeadlessState::InGame(state) => {
                todo!()
            }
        }
    }
}

struct SetupFields {
    seed: Option<Seed>,
    blocked_cells: Option<Vec<usize>>,
    hand_candidates: Option<HandCandidates>,
}
fn parse_setup<'a>(cmd: impl Iterator<Item = &'a str>) -> Result<SetupFields> {
    let mut seed = None;
    let mut blocked_cells: Option<Vec<usize>> = None;
    let mut hand_candidates: Option<HandCandidates> = None;
    for kv in cmd {
        let mut kv = kv.split('=');
        let k = kv.next().unwrap();
        let v = kv.next().unwrap();
        match k {
            "seed" => {
                seed = Some(v.parse()?);
            }
            "blocked_cells" => {
                blocked_cells = Some(parse_blocked_cells(v)?);
            }
            "hand_candidates" => {
                hand_candidates = Some(parse_hand_candidates(v)?);
            }
            _ => panic!("Invalid arg {k}"),
        }
    }
    Ok(SetupFields {
        seed,
        blocked_cells,
        hand_candidates,
    })
}

fn write_setup_ok(
    o: &mut String,
    seed: Seed,
    blocked_cells: impl Iterator<Item = usize>,
    hand_candidates: &HandCandidates,
) -> Result {
    write!(o, "setup-ok")?;
    write!(o, " seed={}", seed)?;
    write!(o, " blocked_cells=")?;
    write_blocked_cells(o, blocked_cells)?;
    write!(o, " hand_candidates=")?;
    write_hand_candidates(o, hand_candidates)?;
    writeln!(o)?;
    Ok(())
}

fn parse_pick_hand<'a>(mut cmd: impl Iterator<Item = &'a str>) -> Result<usize> {
    let mut kv = cmd.next().unwrap().split('=');
    let k = kv.next().unwrap();
    if k == "index" {
        Ok(kv.next().unwrap().parse()?)
    } else {
        panic!("Invalid arg {k}")
    }
}

fn write_pick_hand_ok(o: &mut String) -> Result {
    writeln!(o, "pick-hand-ok")?;
    Ok(())
}

fn write_pick_hand_err(o: &mut String, err: &str) -> Result {
    write!(o, "pick-hand-err")?;
    writeln!(o, " reason=\"{err}\"")?;
    Ok(())
}

fn parse_blocked_cells(s: &str) -> Result<Vec<usize>> {
    let s = &s[1..s.len() - 1]; // remove brackets
    if s.is_empty() {
        return Ok(vec![]);
    }
    s.split(',')
        .map(|v| -> Result<_> { Ok(usize::from_str_radix(v, 16)?) })
        .collect::<Result<Vec<_>>>()
}

fn write_blocked_cells(o: &mut String, mut blocked_cells: impl Iterator<Item = usize>) -> Result {
    write!(o, "[")?;
    if let Some(cell) = blocked_cells.next() {
        write!(o, "{cell:X}")?;
        for cell in blocked_cells {
            write!(o, ",{cell:X}")?;
        }
    }
    write!(o, "]")?;
    Ok(())
}

fn parse_card(s: &str) -> Result<Card> {
    let attack = u8::from_str_radix(&s[0..1], 16)?;
    let card_type = &s[1..2];
    let card_type = match card_type {
        "p" | "P" => CardType::Physical,
        "m" | "M" => CardType::Magical,
        "x" | "X" => CardType::Exploit,
        "a" | "A" => CardType::Assault,
        _ => {
            return Err(Error::InvalidCardType {
                input: card_type.into(),
            })
        }
    };
    let physical_defense = u8::from_str_radix(&s[2..3], 16)?;
    let magical_defense = u8::from_str_radix(&s[3..4], 16)?;
    let arrows = Arrows(u8::from_str_radix(&s[5..], 16)?);
    Ok(Card {
        card_type,
        attack,
        physical_defense,
        magical_defense,
        arrows,
    })
}

fn write_card(o: &mut String, card: Card) -> Result {
    let att = card.attack;
    let phy = card.physical_defense;
    let mag = card.magical_defense;
    let typ = match card.card_type {
        CardType::Physical => 'P',
        CardType::Magical => 'M',
        CardType::Exploit => 'X',
        CardType::Assault => 'A',
    };
    let arr = card.arrows.0;
    write!(o, "{att:X}{typ}{phy:X}{mag:X}@{arr:X}")?;
    Ok(())
}

fn parse_hand_candidate(s: &str) -> Result<HandCandidate> {
    let s = &s[1..s.len() - 1]; // remove brackets
    let mut iter = s.split(',').map(parse_card);
    Ok([
        iter.next().ok_or(Error::HandCandidateTooShort)??,
        iter.next().ok_or(Error::HandCandidateTooShort)??,
        iter.next().ok_or(Error::HandCandidateTooShort)??,
        iter.next().ok_or(Error::HandCandidateTooShort)??,
        iter.next().ok_or(Error::HandCandidateTooShort)??,
    ])
}

fn write_hand_candidate(o: &mut String, hand_candidate: &HandCandidate) -> Result {
    write!(o, "[")?;
    let mut hand_candidate = hand_candidate.iter();
    if let Some(card) = hand_candidate.next() {
        write_card(o, *card)?;
        for card in hand_candidate {
            write!(o, ",")?;
            write_card(o, *card)?;
        }
    }
    write!(o, "]")?;
    Ok(())
}

fn parse_hand_candidates(s: &str) -> Result<HandCandidates> {
    let s = &s[1..s.len() - 1]; // remove brackets
    let mut iter = s.split(';').map(parse_hand_candidate);
    Ok([
        iter.next().ok_or(Error::HandCandidatesTooShort)??,
        iter.next().ok_or(Error::HandCandidatesTooShort)??,
        iter.next().ok_or(Error::HandCandidatesTooShort)??,
    ])
}

fn write_hand_candidates(o: &mut String, hand_candidates: &HandCandidates) -> Result {
    write!(o, "[")?;
    let mut hand_candidates = hand_candidates.iter();
    if let Some(hand) = hand_candidates.next() {
        write_hand_candidate(o, hand)?;
        for hand in hand_candidates {
            write!(o, ";")?;
            write_hand_candidate(o, hand)?;
        }
    }
    write!(o, "]")?;
    Ok(())
}
