use crate::{game_log::GameLog, Card, Cell, GameState, Move, Player};

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
    let attacking_card = hand[next_move.card].take().unwrap();

    game_log.append_place_card(&attacking_card, next_move.cell);

    // handle flips
    for &(idx, arrow) in get_neighbours(next_move.cell).iter() {
        if let Cell::Card {
            owner,
            card: attacked_card,
        } = &mut game_state.board[idx]
        {
            // skip over cards belong to the attacking player
            if *owner == game_state.turn {
                continue;
            }

            // skip if the attacking card doesn't have a arrow in this direction
            if !is_attacking(&attacking_card, arrow) {
                continue;
            }

            if is_defending(attacked_card, arrow) {
                // TODO implement battle
                continue;
            } else {
                // card isn't defending so flip it
                game_log.append_flip_card(attacked_card, idx, game_state.turn);

                *owner = game_state.turn;
            }
        }
    }

    // actually move card onto the board
    game_state.board[next_move.cell] = Cell::Card {
        owner: game_state.turn,
        card: attacking_card,
    };

    // next turn
    game_state.turn = game_state.turn.opposite();

    game_log.next_turn(game_state.turn);

    Ok(())
}

fn is_attacking(attacking_card: &Card, attack_direction: Arrow) -> bool {
    match attack_direction {
        Arrow::TopLeft => attacking_card.arrows.top_left,
        Arrow::Top => attacking_card.arrows.top,
        Arrow::TopRight => attacking_card.arrows.top_right,
        Arrow::Left => attacking_card.arrows.left,
        Arrow::Right => attacking_card.arrows.right,
        Arrow::BottomLeft => attacking_card.arrows.bottom_left,
        Arrow::Bottom => attacking_card.arrows.bottom,
        Arrow::BottomRight => attacking_card.arrows.bottom_right,
    }
}

fn is_defending(attacked_card: &Card, attack_direction: Arrow) -> bool {
    match attack_direction {
        Arrow::TopLeft => attacked_card.arrows.bottom_right,
        Arrow::Top => attacked_card.arrows.bottom,
        Arrow::TopRight => attacked_card.arrows.bottom_left,
        Arrow::Left => attacked_card.arrows.right,
        Arrow::Right => attacked_card.arrows.left,
        Arrow::BottomLeft => attacked_card.arrows.top_right,
        Arrow::Bottom => attacked_card.arrows.top,
        Arrow::BottomRight => attacked_card.arrows.top_left,
    }
}

#[derive(Debug, Clone, Copy)]
enum Arrow {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

// returns index of neighbour cells along with the arrow that points at the neighbour
fn get_neighbours(cell: usize) -> &'static [(usize, Arrow)] {
    use Arrow::*;
    match cell {
        0x0 => &[(0x1, Right), (0x5, BottomRight), (0x4, Bottom)],
        0x1 => &[
            (0x2, Right),
            (0x6, BottomRight),
            (0x5, Bottom),
            (0x4, BottomLeft),
            (0x0, Left),
        ],
        0x2 => &[
            (0x3, Right),
            (0x7, BottomRight),
            (0x6, Bottom),
            (0x5, BottomLeft),
            (0x1, Left),
        ],
        0x3 => &[(0x7, Bottom), (0x6, BottomLeft), (0x2, Left)],
        0x4 => &[
            (0x0, Top),
            (0x1, TopRight),
            (0x5, Right),
            (0x9, BottomRight),
            (0x8, Bottom),
        ],
        0x5 => &[
            (0x1, Top),
            (0x2, TopRight),
            (0x6, Right),
            (0xA, BottomRight),
            (0x9, Bottom),
            (0x8, BottomLeft),
            (0x4, Left),
            (0x0, TopLeft),
        ],
        0x6 => &[
            (0x2, Top),
            (0x3, TopRight),
            (0x7, Right),
            (0xB, BottomRight),
            (0xA, Bottom),
            (0x9, BottomLeft),
            (0x5, Left),
            (0x1, TopLeft),
        ],
        0x7 => &[
            (0x3, Top),
            (0xB, Bottom),
            (0xA, BottomLeft),
            (0x6, Left),
            (0x2, TopLeft),
        ],
        0x8 => &[
            (0x4, Top),
            (0x5, TopRight),
            (0x9, Right),
            (0xD, BottomRight),
            (0xC, Bottom),
        ],
        0x9 => &[
            (0x5, Top),
            (0x6, TopRight),
            (0xA, Right),
            (0xE, BottomRight),
            (0xD, Bottom),
            (0xC, BottomLeft),
            (0x8, Left),
            (0x4, TopLeft),
        ],
        0xA => &[
            (0x6, Top),
            (0x7, TopRight),
            (0xB, Right),
            (0xF, BottomRight),
            (0xE, Bottom),
            (0xD, BottomLeft),
            (0x9, Left),
            (0x5, TopLeft),
        ],
        0xB => &[
            (0x7, Top),
            (0xF, Bottom),
            (0xE, BottomLeft),
            (0xA, Left),
            (0x6, TopLeft),
        ],
        0xC => &[(0x8, Top), (0x9, TopRight), (0xD, Right)],
        0xD => &[
            (0x9, Top),
            (0xA, TopRight),
            (0xE, Right),
            (0xC, Left),
            (0x8, TopLeft),
        ],
        0xE => &[
            (0xA, Top),
            (0xB, TopRight),
            (0xF, Right),
            (0xD, Left),
            (0x9, TopLeft),
        ],
        0xF => &[(0xB, Top), (0xE, Left), (0xA, TopLeft)],
        _ => unreachable!(),
    }
}
