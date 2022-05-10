use crate::{Cell, GameLog, GameLogEntry, GameState, Move, Player};

pub(crate) fn next(
    game_state: &mut GameState,
    game_log: &mut GameLog,
    next_move: Move,
) -> Result<(), String> {
    let hand = match game_state.turn {
        Player::P1 => &mut game_state.p1_hand,
        Player::P2 => &mut game_state.p2_hand,
    };

    // ensure move is valid
    if hand[next_move.card].is_none() {
        return Err(format!("Card {} has already been played", next_move.card));
    }
    if !game_state.board[next_move.cell].is_empty() {
        return Err(format!("Cell {} is not empty", next_move.cell));
    }

    // place card
    let card = hand[next_move.card].take().unwrap();
    let log_entry = GameLogEntry::PlaceCard {
        card: card.clone(),
        cell: next_move.cell,
    };
    game_log.append(game_state.turn, log_entry);
    game_state.board[next_move.cell] = Cell::Card {
        owner: game_state.turn,
        card,
    };

    // next turn
    game_state.turn = match game_state.turn {
        Player::P1 => Player::P2,
        Player::P2 => Player::P1,
    };

    Ok(())
}
