use crate::{
    Arrows, Card, CardType, Cell, GameState, HandCandidate, HandCandidates, PreGameState, Rng, Seed,
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
    InPreGame(PreGameState),
    InGame(GameState),
}

pub(crate) fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();
    let mut in_ = std::io::stdin().lock();

    let mut buf = String::new();

    let mut state = HeadlessState::NotInGame;

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
        match state {
            HeadlessState::NotInGame => {
                if cmd_name == "setup" {
                    let mut seed = None;
                    let mut blocked_cells: Option<Vec<usize>> = None;
                    let mut hand_candidates: Option<HandCandidates> = None;
                    for kv in cmd {
                        let mut kv = kv.split('=');
                        let k = kv.next().unwrap();
                        let v = kv.next().unwrap();
                        match k {
                            "seed" => {
                                seed = Some(parse_seed(v)?);
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

                    let rng = seed.map_or_else(Rng::new, Rng::with_seed);
                    let mut pre_game_state = PreGameState::with_rng(rng);

                    if let Some(blocked_cells) = blocked_cells {
                        pre_game_state.board = Default::default();
                        for cell in blocked_cells {
                            pre_game_state.board[cell] = Cell::Blocked;
                        }
                    }

                    if let Some(hand_candidates) = hand_candidates {
                        pre_game_state.hand_candidates = hand_candidates;
                    }

                    buf.clear();
                    write!(buf, "setup-ok")?;

                    write!(buf, " seed=")?;
                    write_seed(&mut buf, pre_game_state.rng.initial_seed())?;

                    write!(buf, " blocked_cells=")?;
                    let blocked_cells =
                        pre_game_state
                            .board
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, cell)| {
                                if let Cell::Blocked = cell {
                                    Some(idx)
                                } else {
                                    None
                                }
                            });
                    write_blocked_cells(&mut buf, blocked_cells)?;

                    write!(buf, " hand_candidates=")?;
                    write_hand_candidates(&mut buf, &pre_game_state.hand_candidates)?;

                    writeln!(buf)?;

                    out.write_all(buf.as_bytes())?;
                    out.flush()?;

                    state = HeadlessState::InPreGame(pre_game_state);

                    continue;
                }
                panic!("Unexpected command {buf}")
            }
            HeadlessState::InPreGame(state) => {
                todo!()
            }
            HeadlessState::InGame(state) => {
                todo!()
            }
        }
    }
}

fn parse_seed(s: &str) -> Result<Seed> {
    Ok(s.parse()?)
}

fn write_seed(o: &mut String, seed: Seed) -> Result {
    write!(o, "{}", seed)?;
    Ok(())
}

fn parse_blocked_cells(s: &str) -> Result<Vec<usize>> {
    let s = &s[1..s.len() - 1]; // remove brackets
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
