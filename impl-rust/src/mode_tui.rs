use crate::{input, logic, render, render_simple, GameLog, GameState, PreGameState, Rng};

pub(crate) fn run(args: crate::Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{BufRead, Write};

    let mut out = std::io::stdout().lock();
    let mut in_ = std::io::stdin().lock();

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
