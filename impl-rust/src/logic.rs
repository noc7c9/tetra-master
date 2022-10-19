use crate::{
    Arrows, BattleResult, BattleStat, BattleSystem, BattleWinner, Card, CardType, Cell, Entry,
    GameInput, GameInputBattle, GameInputPlace, GameLog, GameState, GameStatus, OwnedCard, Player,
    PreGameInput, PreGameState, PreGameStatus, Rng,
};

#[derive(Debug, PartialEq)]
pub(crate) enum Error {
    InvalidHandPick { hand: usize },
    HandAlreadyPicked { hand: usize },
    CellIsNotEmpty { cell: usize },
    CardAlreadyPlayed { card: usize },
    InvalidBattlePick { cell: usize },
}

pub(crate) fn pre_game_next(
    state: &mut PreGameState,
    log: &mut GameLog,
    input: PreGameInput,
) -> Result<(), Error> {
    match state.status {
        PreGameStatus::P1Picking => {
            if input.pick > 2 {
                return Err(Error::InvalidHandPick { hand: input.pick });
            }

            state.status = PreGameStatus::P2Picking {
                p1_pick: input.pick,
            };
        }
        PreGameStatus::P2Picking { p1_pick } => {
            if input.pick > 2 {
                return Err(Error::InvalidHandPick { hand: input.pick });
            }
            if input.pick == p1_pick {
                return Err(Error::HandAlreadyPicked { hand: input.pick });
            }
            let p2_pick = input.pick;
            state.status = PreGameStatus::Complete { p1_pick, p2_pick };

            log.append(Entry::pre_game_setup(p1_pick, p2_pick));

            // append the first next turn log event to ensure the turn tracking works properly
            log.append(Entry::next_turn(Player::P1));
        }
        _ => unreachable!("next called after pre-game is complete"),
    }

    Ok(())
}

pub(crate) fn game_next(
    state: &mut GameState,
    log: &mut GameLog,
    input: GameInput,
) -> Result<(), Error> {
    match (&state.status, input) {
        (GameStatus::WaitingPlace, GameInput::Place(input)) => {
            handle_waiting_place(state, log, input)
        }
        (GameStatus::WaitingBattle { .. }, GameInput::Battle(input)) => {
            handle_waiting_battle(state, log, input)
        }
        _ => unreachable!("next called with invalid status/input pair"),
    }
}

fn handle_waiting_place(
    state: &mut GameState,
    log: &mut GameLog,
    input: GameInputPlace,
) -> Result<(), Error> {
    let hand_index = input.card;
    let attacker_cell = input.cell;

    let hand = match state.turn {
        Player::P1 => &mut state.p1_hand,
        Player::P2 => &mut state.p2_hand,
    };

    // ensure cell being placed is empty
    if !matches!(state.board[attacker_cell], Cell::Empty) {
        return Err(Error::CellIsNotEmpty {
            cell: attacker_cell,
        });
    }

    // remove the card from the hand
    let attacker = match hand[hand_index].take() {
        None => {
            return Err(Error::CardAlreadyPlayed { card: hand_index });
        }
        Some(card) => OwnedCard {
            owner: state.turn,
            card,
        },
    };

    // place card onto the board
    log.append(Entry::place_card(attacker, attacker_cell));
    state.board[attacker_cell] = Cell::Card(attacker);

    resolve_interactions(state, log, attacker_cell);

    Ok(())
}

fn handle_waiting_battle(
    state: &mut GameState,
    log: &mut GameLog,
    input: GameInputBattle,
) -> Result<(), Error> {
    let defender_cell = input.cell;

    let (attacker_cell, choices) = match &state.status {
        GameStatus::WaitingBattle {
            attacker_cell,
            choices,
        } => (*attacker_cell, choices),
        _ => unreachable!(),
    };

    // ensure input cell is a valid choice
    if choices.iter().all(|&(cell, _)| cell != defender_cell) {
        return Err(Error::InvalidBattlePick {
            cell: defender_cell,
        });
    }

    let winner = battle(state, log, attacker_cell, defender_cell);

    // if the attacker won
    // resolve further interactions
    if winner == BattleWinner::Attacker {
        resolve_interactions(state, log, attacker_cell);
    } else {
        // next turn
        if !check_for_game_over(state) {
            state.turn = state.turn.opposite();
            log.append(Entry::next_turn(state.turn));
        }
    }

    Ok(())
}

// common logic for both handle_waiting_place and handle_waiting_battle
fn resolve_interactions(state: &mut GameState, log: &mut GameLog, attacker_cell: usize) {
    let attacker = match state.board[attacker_cell] {
        Cell::Card(card) => card,
        _ => unreachable!("resolve_interactions can't be called with an invalid attacker_cell"),
    };

    let mut defenders = vec![];
    let mut non_defenders = vec![];
    for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
        let defender = match state.board[defender_cell] {
            Cell::Card(card) => card,
            _ => continue,
        };

        if !does_interact(attacker, defender, arrow) {
            continue;
        }

        if defender.card.arrows.has(arrow.reverse()) {
            defenders.push((defender_cell, defender.card));
        } else {
            non_defenders.push(defender_cell);
        }
    }

    // handle multiple possible battles
    if defenders.len() > 1 {
        defenders.sort_unstable_by_key(|(cell, _)| *cell);
        state.status = GameStatus::WaitingBattle {
            attacker_cell,
            choices: defenders,
        };
        return;
    }

    // handle battles
    let winner = defenders
        .first()
        .map(|(defender_cell, _)| battle(state, log, attacker_cell, *defender_cell));

    // if the attacker won or if there was no battle
    // handle free flips
    if winner == Some(BattleWinner::Attacker) || winner == None {
        for cell in non_defenders {
            let defender = match &mut state.board[cell] {
                Cell::Card(card) => card,
                _ => unreachable!(),
            };
            // skip card if it's already been flipped by a battle
            if defender.owner != attacker.owner {
                flip(log, defender, cell, false);
            }
        }
    }

    // next turn
    if !check_for_game_over(state) {
        state.turn = state.turn.opposite();
        log.append(Entry::next_turn(state.turn));
    }
}

fn check_for_game_over(state: &mut GameState) -> bool {
    if state.p1_hand.iter().all(Option::is_none) && state.p2_hand.iter().all(Option::is_none) {
        let mut p1_cards = 0;
        let mut p2_cards = 0;

        for cell in &state.board {
            if let Cell::Card(OwnedCard { owner, .. }) = cell {
                match owner {
                    Player::P1 => p1_cards += 1,
                    Player::P2 => p2_cards += 1,
                }
            }
        }

        use std::cmp::Ordering;
        let winner = match p1_cards.cmp(&p2_cards) {
            Ordering::Greater => Some(Player::P1),
            Ordering::Less => Some(Player::P2),
            Ordering::Equal => None,
        };

        state.status = GameStatus::GameOver { winner };

        true
    } else {
        state.status = GameStatus::WaitingPlace;

        false
    }
}

fn battle(
    state: &mut GameState,
    log: &mut GameLog,
    attacker_cell: usize,
    defender_cell: usize,
) -> BattleWinner {
    // temporarily take out both cards from the board to allow 2 mut references
    let mut attacker = state.take_card(attacker_cell);
    let mut defender = state.take_card(defender_cell);

    let result = calculate_battle_result(state, attacker.card, defender.card);
    log.append(Entry::battle(
        attacker,
        attacker_cell,
        defender,
        defender_cell,
        result,
    ));
    let (loser_cell, loser) = match result.winner {
        BattleWinner::Defender | BattleWinner::None => {
            // flip attacker
            flip(log, &mut attacker, attacker_cell, false);
            (attacker_cell, attacker)
        }
        BattleWinner::Attacker => {
            // flip defender
            flip(log, &mut defender, defender_cell, false);
            (defender_cell, defender)
        }
    };

    // combo flip any cards the losing card points at
    for &(comboed_cell, arrow) in get_possible_neighbours(loser_cell) {
        let comboed = match &mut state.board[comboed_cell] {
            Cell::Card(card) => card,
            _ => continue,
        };

        if !does_interact(loser, *comboed, arrow) {
            continue;
        }

        flip(log, comboed, comboed_cell, true);
    }

    // place both cards back into the board
    state.board[attacker_cell] = Cell::Card(attacker);
    state.board[defender_cell] = Cell::Card(defender);

    result.winner
}

fn flip(log: &mut GameLog, card: &mut OwnedCard, cell: usize, via_combo: bool) {
    let to = card.owner.opposite();
    log.append(Entry::flip_card(*card, cell, to, via_combo));
    card.owner = to;
}

fn roll(battle_system: &mut BattleSystem, rng: &mut Rng, value: u8) -> u8 {
    match battle_system {
        BattleSystem::Original => {
            let min = value << 4; // range: 00, 10, 20, ..., F0
            let max = min | 0xF; // range: 0F, 1F, 2F, ..., FF

            let stat1 = rng.u8(min..=max);
            let stat2 = rng.u8(0..=stat1);
            stat1 - stat2
        }
        BattleSystem::Dice { sides } => {
            // roll {value} dice and return the sum
            (0..value).map(|_| rng.u8(1..=*sides)).sum()
        }
        // rolls are proportional to the rng number and falls in the range 0x00 - 0x{value}F
        // meant for making battles in tests predictable
        BattleSystem::Test => {
            let max = (value << 4) | 0xF;
            let rng = rng.u8(..) as f64 / 0xFF as f64;
            (rng * max as f64).round() as u8
        }
    }
}

fn get_attack_stat(rng: &mut Rng, battle_system: &mut BattleSystem, attacker: Card) -> BattleStat {
    let (digit, value) = if let CardType::Assault = attacker.card_type {
        // use the highest stat
        let att = attacker.attack;
        let phy = attacker.physical_defense;
        let mag = attacker.magical_defense;
        if mag > att && mag > phy {
            (3, mag)
        } else if phy > att {
            (2, phy)
        } else {
            (0, att)
        }
    } else {
        // otherwise use the attack stat
        (0, attacker.attack)
    };

    let roll = roll(battle_system, rng, value);
    BattleStat { digit, value, roll }
}

fn get_defense_stat(
    rng: &mut Rng,
    battle_system: &mut BattleSystem,
    attacker: Card,
    defender: Card,
) -> BattleStat {
    let (digit, value) = match attacker.card_type {
        CardType::Physical => (2, defender.physical_defense),
        CardType::Magical => (3, defender.magical_defense),
        CardType::Exploit => {
            // use the lowest defense stat
            if defender.physical_defense < defender.magical_defense {
                (2, defender.physical_defense)
            } else {
                (3, defender.magical_defense)
            }
        }
        CardType::Assault => {
            // use the lowest stat
            let att = defender.attack;
            let phy = defender.physical_defense;
            let mag = defender.magical_defense;
            if att < phy && att < mag {
                (0, att)
            } else if phy < mag {
                (2, phy)
            } else {
                (3, mag)
            }
        }
    };

    let roll = roll(battle_system, rng, value);
    BattleStat { digit, value, roll }
}

fn calculate_battle_result(state: &mut GameState, attacker: Card, defender: Card) -> BattleResult {
    let battle_system = &mut state.battle_system;

    let attack_stat = get_attack_stat(&mut state.rng, battle_system, attacker);
    let defense_stat = get_defense_stat(&mut state.rng, battle_system, attacker, defender);

    use std::cmp::Ordering;
    let winner = match attack_stat.roll.cmp(&defense_stat.roll) {
        Ordering::Greater => BattleWinner::Attacker,
        Ordering::Less => BattleWinner::Defender,
        Ordering::Equal => BattleWinner::None,
    };

    BattleResult {
        winner,
        attack_stat,
        defense_stat,
    }
}

fn does_interact(attacker: OwnedCard, defender: OwnedCard, arrow_to_defender: Arrows) -> bool {
    // they don't interact if both cards belong to the same player
    if defender.owner == attacker.owner {
        return false;
    }

    // they interact if the attacking card has an arrow in the direction of the defender
    attacker.card.arrows.has(arrow_to_defender)
}

// returns neighbouring cells along with the arrow that points at them
fn get_possible_neighbours(cell: usize) -> &'static [(usize, Arrows)] {
    const U: Arrows = Arrows::UP;
    const UR: Arrows = Arrows::UP_RIGHT;
    const R: Arrows = Arrows::RIGHT;
    const DR: Arrows = Arrows::DOWN_RIGHT;
    const D: Arrows = Arrows::DOWN;
    const DL: Arrows = Arrows::DOWN_LEFT;
    const L: Arrows = Arrows::LEFT;
    const UL: Arrows = Arrows::UP_LEFT;
    match cell {
        0x0 => &[(0x1, R), (0x4, D), (0x5, DR)],
        0x1 => &[(0x0, L), (0x2, R), (0x4, DL), (0x5, D), (0x6, DR)],
        0x2 => &[(0x1, L), (0x3, R), (0x5, DL), (0x6, D), (0x7, DR)],
        0x3 => &[(0x2, L), (0x6, DL), (0x7, D)],
        0x4 => &[(0x0, U), (0x1, UR), (0x5, R), (0x8, D), (0x9, DR)],
        0x5 => &[
            (0x0, UL),
            (0x1, U),
            (0x2, UR),
            (0x4, L),
            (0x6, R),
            (0x8, DL),
            (0x9, D),
            (0xA, DR),
        ],
        0x6 => &[
            (0x1, UL),
            (0x2, U),
            (0x3, UR),
            (0x5, L),
            (0x7, R),
            (0x9, DL),
            (0xA, D),
            (0xB, DR),
        ],
        0x7 => &[(0x3, U), (0xB, D), (0xA, DL), (0x6, L), (0x2, UL)],
        0x8 => &[(0x4, U), (0x5, UR), (0x9, R), (0xD, DR), (0xC, D)],
        0x9 => &[
            (0x5, U),
            (0x6, UR),
            (0xA, R),
            (0xE, DR),
            (0xD, D),
            (0xC, DL),
            (0x8, L),
            (0x4, UL),
        ],
        0xA => &[
            (0x6, U),
            (0x7, UR),
            (0xB, R),
            (0xF, DR),
            (0xE, D),
            (0xD, DL),
            (0x9, L),
            (0x5, UL),
        ],
        0xB => &[(0x6, UL), (0x7, U), (0xA, L), (0xE, DL), (0xF, D)],
        0xC => &[(0x8, U), (0x9, UR), (0xD, R)],
        0xD => &[(0x8, UL), (0x9, U), (0xA, UR), (0xC, L), (0xE, R)],
        0xE => &[(0x9, UL), (0xA, U), (0xB, UR), (0xD, L), (0xF, R)],
        0xF => &[(0xA, UL), (0xB, U), (0xE, L)],
        _ => unreachable!(),
    }
}