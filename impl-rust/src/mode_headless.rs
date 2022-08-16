use crate::{
    Arrows, Card, CardType, Cell, GameState, HandCandidate, HandCandidates, PreGameState, Rng,
};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum HeadlessState {
    NotInGame,
    InPreGame(PreGameState),
    InGame(GameState),
}

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    macro_rules! log {
        () => {
            // eprintln!();
        };
        ($($es:expr),+) => {{
            // eprint!("IMPL: ");
            // eprintln!($($es),+);
        }};
    }

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

        let mut cmd = buf.split(' ');
        let cmd_name = cmd.next().unwrap();

        log!("GOT CMD: {cmd:?}");

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
                                seed = Some(v.parse().unwrap());
                            }
                            "blocked_cells" => {
                                let v = &v[1..v.len() - 1];
                                blocked_cells = Some(
                                    v.split(',')
                                        .map(|v| usize::from_str_radix(v, 16).unwrap())
                                        .collect(),
                                );
                            }
                            "hand_candidates" => {
                                let v = &v[1..v.len() - 1];
                                let mut iter = v.split(';').map(|hand| -> HandCandidate {
                                    let hand = &hand[1..hand.len() - 1];
                                    let mut iter = hand.split(',').map(deserialize_card);
                                    [
                                        iter.next().unwrap(),
                                        iter.next().unwrap(),
                                        iter.next().unwrap(),
                                        iter.next().unwrap(),
                                        iter.next().unwrap(),
                                    ]
                                });
                                hand_candidates = Some([
                                    iter.next().unwrap(),
                                    iter.next().unwrap(),
                                    iter.next().unwrap(),
                                ]);
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

                    let seed = pre_game_state.rng.initial_seed();
                    let blocked_cells = pre_game_state
                        .board
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, cell)| {
                            if let Cell::Blocked = cell {
                                Some(format!("{idx:X}"))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let hand_candidates = pre_game_state
                        .hand_candidates
                        .iter()
                        .map(|hand| {
                            let serialized = hand
                                .iter()
                                .map(|card| {
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
                                    format!("{att:X}{typ}{phy:X}{mag:X}@{arr:X}")
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            format!("[{serialized}]")
                        })
                        .collect::<Vec<_>>()
                        .join(";");
                    writeln!(
                        out,
                        "setup-ok seed={seed} blocked_cells=[{blocked_cells}] hand_candidates=[{hand_candidates}]"
                    )?;

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

fn deserialize_card(s: &str) -> Card {
    let attack = u8::from_str_radix(&s[0..1], 16).unwrap();
    let card_type = match &s[1..2] {
        "p" | "P" => CardType::Physical,
        "m" | "M" => CardType::Magical,
        "x" | "X" => CardType::Exploit,
        "a" | "A" => CardType::Assault,
        _ => unreachable!(),
    };
    let physical_defense = u8::from_str_radix(&s[2..3], 16).unwrap();
    let magical_defense = u8::from_str_radix(&s[3..4], 16).unwrap();
    let arrows = Arrows(u8::from_str_radix(&s[5..], 16).unwrap());
    Card {
        card_type,
        attack,
        physical_defense,
        magical_defense,
        arrows,
    }
}
