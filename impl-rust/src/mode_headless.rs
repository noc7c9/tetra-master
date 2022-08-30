use crate::{
    logic, Arrows, BattleStat, BattleSystem, BattleWinner, Board, Card, CardType, Cell, Entry,
    GameInput, GameInputBattle, GameInputPlace, GameLog, GameState, GameStatus, HandCandidate,
    HandCandidates, Player, PreGameInput, PreGameState, Rng, Seed, Sexpr, Token,
};
use std::fmt::Write;

#[derive(Debug)]
enum Error {
    HandCandidatesTooShort,
    HandCandidateTooShort,
    InvalidRng { input: String },
    InvalidBattleSystem { input: String },
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
            InvalidRng { input } => write!(f, "'{input}' is not a valid rng"),
            InvalidBattleSystem { input } => write!(f, "'{input}' is not a valid battle system"),
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

        let mut cmd = Sexpr::new(&buf);
        cmd.list_start();
        let cmd_name = cmd.atom();

        if cmd_name == "quit" {
            return Ok(());
        }

        // handle command
        match current_state {
            HeadlessState::NotInGame => {
                if cmd_name == "setup" {
                    let setup = parse::setup(cmd)?;

                    let battle_system = setup.battle_system.unwrap_or(BattleSystem::Original);

                    let state = PreGameState::builder()
                        .rng(setup.rng)
                        .hand_candidates(setup.hand_candidates)
                        .board(setup.blocked_cells.map(|blocked_cells| {
                            let mut board: Board = Default::default();
                            for cell in blocked_cells {
                                board[cell] = Cell::Blocked;
                            }
                            board
                        }))
                        .build();

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
                    write::setup_ok(
                        &mut buf,
                        seed,
                        &battle_system,
                        blocked_cells,
                        hand_candidates,
                    )?;

                    out.write_all(buf.as_bytes())?;
                    out.flush()?;

                    current_state = HeadlessState::InPreGame(state, battle_system);

                    continue;
                }
                panic!("Unexpected command {buf}")
            }
            HeadlessState::InPreGame(mut state, battle_system) => {
                if cmd_name == "pick-hand" {
                    let hand = parse::pick_hand(cmd)?;

                    buf.clear();

                    let input = PreGameInput { pick: hand };
                    if let Err(err) = logic::pre_game_next(&mut state, &mut log, input) {
                        write::pick_hand_err(&mut buf, &err)?;
                    } else {
                        write::pick_hand_ok(&mut buf)?;
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
            HeadlessState::InGame(mut state) => {
                let input = if cmd_name == "place-card" {
                    let (card, cell) = parse::place_card(cmd)?;
                    GameInput::Place(GameInputPlace { card, cell })
                } else if cmd_name == "pick-battle" {
                    let cell = parse::pick_battle(cmd)?;
                    GameInput::Battle(GameInputBattle { cell })
                } else {
                    panic!("Unexpected command {buf}")
                };

                buf.clear();

                if let Err(_err) = logic::game_next(&mut state, &mut log, input) {
                    todo!();
                    // write_pick_hand_err(&mut buf, &err)?;
                } else if let GameStatus::WaitingBattle { choices, .. } = &state.status {
                    write::place_card_pick_battle(&mut buf, choices)?;
                } else {
                    write::place_card_ok(&mut buf, log.new_entries(), &state.status)?;
                }

                out.write_all(buf.as_bytes())?;
                out.flush()?;

                // restore previous state to keep borrow checker happy
                current_state = HeadlessState::InGame(state);
                continue;
            }
        }
    }
}

mod parse {
    use super::*;

    fn prop<T>(cmd: &mut Sexpr, expected_name: &'static str, f: impl FnOnce(&str) -> T) -> T {
        cmd.list_start();
        let name = cmd.atom();
        if name == expected_name {
            let value = f(cmd.atom());
            cmd.list_end();
            value
        } else {
            panic!("Invalid property: {name}")
        }
    }

    #[derive(Debug)]
    pub(super) struct Setup {
        pub(super) rng: Option<Rng>,
        pub(super) battle_system: Option<BattleSystem>,
        pub(super) blocked_cells: Option<Vec<usize>>,
        pub(super) hand_candidates: Option<HandCandidates>,
    }
    pub(super) fn setup(mut cmd: Sexpr) -> Result<Setup> {
        let mut rng = None;
        let mut battle_system = None;
        let mut blocked_cells = None;
        let mut hand_candidates = None;
        loop {
            match cmd.next() {
                Some(Token::ListEnd) => break,
                Some(Token::ListStart) => {
                    let arg = cmd.atom();
                    match arg {
                        "rng" => {
                            rng = Some(self::rng(&mut cmd)?);
                        }
                        "battle-system" => {
                            battle_system = Some(self::battle_system(&mut cmd)?);
                        }
                        "blocked-cells" => {
                            blocked_cells = Some(self::blocked_cells(&mut cmd)?);
                        }
                        "hand-candidates" => {
                            hand_candidates = Some(self::hand_candidates(&mut cmd)?);
                        }
                        _ => panic!("Invalid arg {arg}"),
                    }
                    cmd.list_end();
                }
                _ => unreachable!(),
            }
        }
        Ok(Setup {
            rng,
            battle_system,
            blocked_cells,
            hand_candidates,
        })
    }

    pub(super) fn pick_hand(mut cmd: Sexpr) -> Result<usize> {
        let hand = prop(&mut cmd, "hand", |v| v.parse())?;
        Ok(hand)
    }

    pub(super) fn place_card(mut cmd: Sexpr) -> Result<(usize, usize)> {
        let card = prop(&mut cmd, "card", |v| v.parse())?;
        let cell = prop(&mut cmd, "cell", |v| usize::from_str_radix(v, 16))?;
        Ok((card, cell))
    }

    pub(super) fn pick_battle(mut cmd: Sexpr) -> Result<usize> {
        let cell = prop(&mut cmd, "cell", |v| usize::from_str_radix(v, 16))?;
        Ok(cell)
    }

    fn rng(cmd: &mut Sexpr) -> Result<Rng> {
        let kind = cmd.atom();
        match kind {
            "seed" => {
                let seed = u64::from_str_radix(cmd.atom(), 16)?;
                Ok(Rng::with_seed(seed))
            }
            "external" => {
                cmd.list_start();
                let mut rolls = std::collections::VecDeque::new();
                while let Some(Token::Atom(v)) = cmd.next() {
                    rolls.push_back(u8::from_str_radix(v, 16)?);
                }
                Ok(Rng::new_external(rolls))
            }
            _ => Err(Error::InvalidRng { input: kind.into() }),
        }
    }

    fn battle_system(cmd: &mut Sexpr) -> Result<BattleSystem> {
        let kind = cmd.atom();
        match kind {
            "original" => Ok(BattleSystem::Original),
            "original-approx" => Ok(BattleSystem::OriginalApprox),
            "dice" => {
                let sides = u8::from_str_radix(cmd.atom(), 16)?;
                Ok(BattleSystem::Dice { sides })
            }
            _ => Err(Error::InvalidBattleSystem { input: kind.into() }),
        }
    }

    fn blocked_cells(cmd: &mut Sexpr) -> Result<Vec<usize>> {
        cmd.list_start();
        let mut cells = Vec::new();
        while let Some(Token::Atom(v)) = cmd.next() {
            cells.push(usize::from_str_radix(v, 16)?);
        }
        Ok(cells)
    }

    fn card(cmd: &mut Sexpr) -> Result<Card> {
        let s = cmd.atom();
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

    fn hand_candidate(cmd: &mut Sexpr) -> Result<HandCandidate> {
        cmd.list_start();
        let hand_candidate = [card(cmd)?, card(cmd)?, card(cmd)?, card(cmd)?, card(cmd)?];
        cmd.list_end();
        Ok(hand_candidate)
    }

    fn hand_candidates(cmd: &mut Sexpr) -> Result<HandCandidates> {
        cmd.list_start();
        let hand_candidates = [
            hand_candidate(cmd)?,
            hand_candidate(cmd)?,
            hand_candidate(cmd)?,
        ];
        cmd.list_end();
        Ok(hand_candidates)
    }
}

mod write {
    use super::*;

    pub(super) fn setup_ok(
        o: &mut String,
        seed: Option<Seed>,
        battle_system: &BattleSystem,
        blocked_cells: impl Iterator<Item = usize>,
        hand_candidates: &HandCandidates,
    ) -> Result {
        write!(o, "(setup-ok")?;

        if let Some(seed) = seed {
            write!(o, " (seed {seed:X})")?;
        }

        write!(o, " (battle-system ")?;
        write::battle_system(o, battle_system)?;

        write!(o, ") (blocked-cells ")?;
        write::blocked_cells(o, blocked_cells)?;

        write!(o, ") (hand-candidates ")?;
        write::hand_candidates(o, hand_candidates)?;

        writeln!(o, "))")?;
        Ok(())
    }

    pub(super) fn pick_hand_ok(o: &mut String) -> Result {
        writeln!(o, "(pick-hand-ok)")?;
        Ok(())
    }

    pub(super) fn pick_hand_err(o: &mut String, err: &str) -> Result {
        writeln!(o, "(pick-hand-err (reason {err:?}))")?;
        Ok(())
    }

    pub(super) fn place_card_ok(o: &mut String, entries: &[Entry], status: &GameStatus) -> Result {
        write!(o, "(place-card-ok")?;

        for entry in entries {
            match entry {
                Entry::FlipCard {
                    cell, via_combo, ..
                } => {
                    if *via_combo {
                        write!(o, " (combo-flip {cell:X})")?;
                    } else {
                        write!(o, " (flip {cell:X})")?;
                    }
                }
                Entry::Battle {
                    result,
                    attacker_cell,
                    defender_cell,
                    ..
                } => {
                    write!(o, " (battle ")?;
                    battler(o, *attacker_cell, result.attack_stat)?;
                    write!(o, " ")?;
                    battler(o, *defender_cell, result.defense_stat)?;
                    write!(o, " ")?;
                    battle_winner(o, result.winner)?;
                    write!(o, ")")?;
                }
                _ => {}
            }
        }

        if let GameStatus::GameOver { winner } = status {
            write!(o, " (game-over ")?;
            let winner = match winner {
                Some(Player::P1) => "player1",
                Some(Player::P2) => "player2",
                None => "draw",
            };
            write!(o, "{winner})")?;
        }

        writeln!(o, ")")?;
        Ok(())
    }

    pub(super) fn place_card_pick_battle(o: &mut String, choices: &[(usize, Card)]) -> Result {
        write!(o, "(place-card-pick-battle (choices (")?;
        let mut choices = choices.iter();
        if let Some((choice, _)) = choices.next() {
            write!(o, "{choice:X}")?;
            for (choice, _) in choices {
                write!(o, " {choice:X}")?;
            }
        }
        writeln!(o, ")))")?;
        Ok(())
    }

    fn battle_system(o: &mut String, battle_system: &BattleSystem) -> Result {
        match battle_system {
            BattleSystem::Original => write!(o, "original")?,
            BattleSystem::OriginalApprox => write!(o, "original-approx")?,
            BattleSystem::Dice { sides } => write!(o, "dice {sides:X}")?,
        }
        Ok(())
    }

    fn blocked_cells(o: &mut String, mut blocked_cells: impl Iterator<Item = usize>) -> Result {
        write!(o, "(")?;
        if let Some(cell) = blocked_cells.next() {
            write!(o, "{cell:X}")?;
            for cell in blocked_cells {
                write!(o, " {cell:X}")?;
            }
        }
        write!(o, ")")?;
        Ok(())
    }

    fn card(o: &mut String, card: Card) -> Result {
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
        write!(o, "{att:X}{typ}{phy:X}{mag:X}_{arr:X}")?;
        Ok(())
    }

    fn hand_candidate(o: &mut String, hand_candidate: &HandCandidate) -> Result {
        write!(o, "(")?;
        let mut hand_candidate = hand_candidate.iter();
        if let Some(card) = hand_candidate.next() {
            self::card(o, *card)?;
            for card in hand_candidate {
                write!(o, " ")?;
                self::card(o, *card)?;
            }
        }
        write!(o, ")")?;
        Ok(())
    }

    fn hand_candidates(o: &mut String, hand_candidates: &HandCandidates) -> Result {
        write!(o, "(")?;
        let mut hand_candidates = hand_candidates.iter();
        if let Some(hand) = hand_candidates.next() {
            hand_candidate(o, hand)?;
            for hand in hand_candidates {
                write!(o, " ")?;
                hand_candidate(o, hand)?;
            }
        }
        write!(o, ")")?;
        Ok(())
    }

    fn battler(o: &mut String, cell: usize, stat: BattleStat) -> Result {
        let BattleStat { digit, value, roll } = stat;
        let digit = match digit {
            0 => "A",
            2 => "P",
            3 => "M",
            _ => unreachable!(),
        };
        write!(o, "({cell:X} {digit} {value:X} {roll:X})")?;
        Ok(())
    }

    fn battle_winner(o: &mut String, winner: BattleWinner) -> Result {
        let winner = match winner {
            BattleWinner::Attacker => "attacker",
            BattleWinner::Defender => "defender",
            BattleWinner::None => "none",
        };
        write!(o, "{winner}")?;
        Ok(())
    }
}
