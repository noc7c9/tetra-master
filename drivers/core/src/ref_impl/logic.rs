use crate::{
    command,
    response::{self, ErrorResponse, RandomNumberRequest, ResolveBattle},
    Arrows, BattleSystem, BattleWinner, Battler, BoardCells, Card, CardType, Digit, Event, Player,
    BOARD_SIZE, HAND_SIZE,
};

type Hand = [Option<Card>; HAND_SIZE];
type Board = [Cell; BOARD_SIZE];

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

pub struct State {
    battle_system: BattleSystem,
    turn: Player,
    board: Board,
    hand_blue: Hand,
    hand_red: Hand,
    status: Status,
    events: Vec<Event>,
}

enum Status {
    WaitingPlaceCard,
    WaitingResolveBattle(WaitingResolveBattle),
    WaitingPickBattle {
        attacker_cell: u8,
        choices: BoardCells,
    },
    GameOver,
}

struct WaitingResolveBattle {
    attacker: BattlerWaitingResolve,
    defender: BattlerWaitingResolve,
}

#[derive(Debug, Copy, Clone)]
pub struct BattlerWaitingResolve {
    cell: u8,
    digit: Digit,
    value: u8,
    roll: RandomNumberRequest,
}

impl BattlerWaitingResolve {
    fn resolve(
        self,
        battle_system: BattleSystem,
        mut numbers: Vec<u8>,
    ) -> Result<Battler, ErrorResponse> {
        macro_rules! get_number {
            ($numbers:expr) => {
                match $numbers.pop() {
                    None => return Err(ErrorResponse::NotEnoughNumbersInResolve),
                    Some(num) => num,
                }
            };
            ($numbers:expr, $range:expr) => {
                map_number_to_range(get_number!($numbers), $range)
            };
        }

        let roll = match battle_system {
            BattleSystem::Original => {
                let min = self.value << 4; // range: 00, 10, 20, ..., F0
                let max = min | 0xF; // range: 0F, 1F, 2F, ..., FF

                let stat1 = get_number!(numbers, min..=max);
                let stat2 = get_number!(numbers, ..=stat1);
                stat1 - stat2
            }
            BattleSystem::Dice { .. } => {
                // roll {value} dice and return the sum
                let mut sum = 0;
                for _ in 0..self.value {
                    sum += get_number!(numbers);
                }
                sum
            }
            BattleSystem::Deterministic => self.value,
            BattleSystem::Test => get_number!(numbers),
        };

        Ok(Battler {
            cell: self.cell,
            digit: self.digit,
            value: self.value,
            roll,
        })
    }
}

impl State {
    pub(super) fn new(cmd: &command::Setup) -> Self {
        fn convert_hand([a, b, c, d, e]: crate::Hand) -> Hand {
            [Some(a), Some(b), Some(c), Some(d), Some(e)]
        }

        let mut board: Board = [Cell::Empty; BOARD_SIZE];
        for cell in cmd.blocked_cells {
            board[cell as usize] = Cell::Blocked;
        }

        Self {
            battle_system: cmd.battle_system,
            turn: cmd.starting_player,
            board,
            hand_blue: convert_hand(cmd.hand_blue),
            hand_red: convert_hand(cmd.hand_red),
            status: Status::WaitingPlaceCard,
            events: Vec::new(),
        }
    }

    pub(super) fn handle_place_card(
        &mut self,
        cmd: command::PlaceCard,
    ) -> Result<response::PlayOk, ErrorResponse> {
        if let Status::WaitingPlaceCard = self.status {
            self.assert_command_player(cmd.player);

            // ensure cell being placed is empty
            if self.board[cmd.cell as usize] != Cell::Empty {
                return Err(ErrorResponse::CellIsNotEmpty { cell: cmd.cell });
            }

            let hand = match self.turn {
                Player::Blue => &mut self.hand_blue,
                Player::Red => &mut self.hand_red,
            };

            // remove the card from the hand
            let card = match hand[cmd.card as usize].take() {
                Some(card) => card,
                None => {
                    return Err(ErrorResponse::CardAlreadyPlayed { card: cmd.card });
                }
            };

            // place card onto the board
            let owner = self.turn;
            self.board[cmd.cell as usize] = Cell::Card(OwnedCard { owner, card });

            self.resolve_interactions(cmd.cell);

            Ok(self.create_response())
        } else {
            Err(ErrorResponse::InvalidCommandForState)
        }
    }

    pub(super) fn handle_resolve_battle(
        &mut self,
        cmd: command::ResolveBattle,
    ) -> Result<response::PlayOk, ErrorResponse> {
        if let Status::WaitingResolveBattle(ref status) = self.status {
            let attacker = status
                .attacker
                .resolve(self.battle_system, cmd.attack_roll)?;
            let defender = status
                .defender
                .resolve(self.battle_system, cmd.defend_roll)?;

            use std::cmp::Ordering;
            let winner = match attacker.roll.cmp(&defender.roll) {
                Ordering::Greater => BattleWinner::Attacker,
                Ordering::Less => BattleWinner::Defender,
                Ordering::Equal => BattleWinner::None,
            };

            // send battle event
            self.events.push(Event::Battle {
                winner,
                attacker,
                defender,
            });

            // flip losing card
            let loser_cell = match winner {
                BattleWinner::Defender | BattleWinner::None => {
                    self.flip(attacker.cell, false);
                    attacker.cell
                }
                BattleWinner::Attacker => {
                    self.flip(defender.cell, false);
                    defender.cell
                }
            };

            // combo flip any cards the losing card points at
            for &(comboed_cell, arrow) in get_possible_neighbours(loser_cell) {
                let loser = match &self.board[loser_cell as usize] {
                    Cell::Card(card) => card,
                    _ => unreachable!(),
                };
                let comboed = match &self.board[comboed_cell as usize] {
                    Cell::Card(card) => card,
                    _ => continue,
                };

                if !does_interact(*loser, *comboed, arrow) {
                    continue;
                }

                self.flip(comboed_cell, true);
            }

            // if the attacker won
            // resolve further interactions
            if winner == BattleWinner::Attacker {
                self.resolve_interactions(attacker.cell);
            } else {
                // next turn
                if !self.is_game_over() {
                    self.end_turn();
                }
            }

            Ok(self.create_response())
        } else {
            Err(ErrorResponse::InvalidCommandForState)
        }
    }

    pub(super) fn handle_pick_battle(
        &mut self,
        cmd: command::PickBattle,
    ) -> Result<response::PlayOk, ErrorResponse> {
        if let Status::WaitingPickBattle {
            attacker_cell,
            choices,
        } = self.status
        {
            self.assert_command_player(cmd.player);

            let defender_cell = cmd.cell;

            // ensure input cell is a valid choice
            if choices.into_iter().all(|cell| cell != defender_cell) {
                return Err(ErrorResponse::InvalidBattlePick {
                    cell: defender_cell,
                });
            }

            self.resolve_battle(attacker_cell, defender_cell);

            Ok(self.create_response())
        } else {
            Err(ErrorResponse::InvalidCommandForState)
        }
    }

    fn create_response(&mut self) -> response::PlayOk {
        let (resolve_battle, pick_battle) = match &self.status {
            Status::WaitingPlaceCard | Status::GameOver { .. } => (None, BoardCells::NONE),
            Status::WaitingResolveBattle(status) => (
                Some(ResolveBattle {
                    attack_roll: status.attacker.roll,
                    defend_roll: status.defender.roll,
                }),
                BoardCells::NONE,
            ),
            Status::WaitingPickBattle { choices, .. } => (None, *choices),
        };
        response::PlayOk {
            resolve_battle,
            pick_battle,
            events: std::mem::take(&mut self.events),
        }
    }

    fn assert_command_player(&mut self, player: Player) {
        assert!(
            player == self.turn,
            "Unexpected player ({}) played move, expected move by {}",
            player,
            self.turn,
        );
    }

    fn end_turn(&mut self) {
        self.turn = self.turn.opposite();
        self.events.push(Event::NextTurn { to: self.turn });
    }

    fn resolve_interactions(&mut self, attacker_cell: u8) {
        let attacker = match self.board[attacker_cell as usize] {
            Cell::Card(card) => card,
            _ => unreachable!(),
        };

        let mut defenders = BoardCells::NONE;
        let mut non_defenders = BoardCells::NONE;
        for &(defender_cell, arrow) in get_possible_neighbours(attacker_cell) {
            let defender = match self.board[defender_cell as usize] {
                Cell::Card(card) => card,
                _ => continue,
            };

            if !does_interact(attacker, defender, arrow) {
                continue;
            }

            if defender.card.arrows.has_any(arrow.reverse()) {
                defenders.set(defender_cell);
            } else {
                non_defenders.set(defender_cell);
            }
        }

        match defenders.len() {
            0 => {
                // no battles, flip non-defenders
                for cell in non_defenders {
                    self.flip(cell, false);
                }

                // no more interactions found, next turn
                if !self.is_game_over() {
                    self.end_turn()
                }
            }
            1 => {
                // handle battle
                let defender_cell = defenders.into_iter().next().unwrap();
                self.resolve_battle(attacker_cell, defender_cell);
            }
            _ => {
                // handle multiple possible battles
                self.status = Status::WaitingPickBattle {
                    attacker_cell,
                    choices: defenders,
                };
            }
        }
    }

    fn flip(&mut self, cell: u8, via_combo: bool) {
        let card = match &mut self.board[cell as usize] {
            Cell::Card(card) => card,
            _ => unreachable!(),
        };
        card.owner = card.owner.opposite();

        self.events.push(if via_combo {
            Event::ComboFlip { cell }
        } else {
            Event::Flip { cell }
        });
    }

    fn resolve_battle(&mut self, attacker_cell: u8, defender_cell: u8) {
        let attacker = match &self.board[attacker_cell as usize] {
            Cell::Card(owned) => owned.card,
            _ => unreachable!(),
        };
        let defender = match &self.board[defender_cell as usize] {
            Cell::Card(owned) => owned.card,
            _ => unreachable!(),
        };

        let (attacker_digit, attacker_value) = get_attack_stat(attacker);
        let (defender_digit, defender_value) = get_defend_stat(attacker, defender);

        let attacker_roll = get_random_number_request(self.battle_system, attacker_value);
        let defender_roll = get_random_number_request(self.battle_system, defender_value);

        self.status = Status::WaitingResolveBattle(WaitingResolveBattle {
            attacker: BattlerWaitingResolve {
                cell: attacker_cell,
                digit: attacker_digit,
                value: attacker_value,
                roll: attacker_roll,
            },
            defender: BattlerWaitingResolve {
                cell: defender_cell,
                digit: defender_digit,
                value: defender_value,
                roll: defender_roll,
            },
        });
    }

    fn is_game_over(&mut self) -> bool {
        if self.hand_blue.iter().all(Option::is_none) && self.hand_red.iter().all(Option::is_none) {
            let mut blue_cards = 0;
            let mut red_cards = 0;

            for cell in &self.board {
                if let Cell::Card(OwnedCard { owner, .. }) = cell {
                    match owner {
                        Player::Blue => blue_cards += 1,
                        Player::Red => red_cards += 1,
                    }
                }
            }

            use std::cmp::Ordering;
            let winner = match blue_cards.cmp(&red_cards) {
                Ordering::Greater => Some(Player::Blue),
                Ordering::Less => Some(Player::Red),
                Ordering::Equal => None,
            };

            self.status = Status::GameOver;
            self.events.push(Event::GameOver { winner });

            true
        } else {
            self.status = Status::WaitingPlaceCard;

            false
        }
    }
}

fn get_attack_stat(attacker: Card) -> (Digit, u8) {
    if let CardType::Assault = attacker.card_type {
        // use the highest stat
        let att = attacker.attack;
        let phy = attacker.physical_defense;
        let mag = attacker.magical_defense;
        if mag > att && mag > phy {
            (Digit::MagicalDefense, mag)
        } else if phy > att {
            (Digit::PhysicalDefense, phy)
        } else {
            (Digit::Attack, att)
        }
    } else {
        // otherwise use the attack stat
        (Digit::Attack, attacker.attack)
    }
}

fn get_defend_stat(attacker: Card, defender: Card) -> (Digit, u8) {
    match attacker.card_type {
        CardType::Physical => (Digit::PhysicalDefense, defender.physical_defense),
        CardType::Magical => (Digit::MagicalDefense, defender.magical_defense),
        CardType::Exploit => {
            // use the lowest defense stat
            if defender.physical_defense < defender.magical_defense {
                (Digit::PhysicalDefense, defender.physical_defense)
            } else {
                (Digit::MagicalDefense, defender.magical_defense)
            }
        }
        CardType::Assault => {
            // use the lowest stat
            let att = defender.attack;
            let phy = defender.physical_defense;
            let mag = defender.magical_defense;
            if att < phy && att < mag {
                (Digit::Attack, att)
            } else if phy < mag {
                (Digit::PhysicalDefense, phy)
            } else {
                (Digit::MagicalDefense, mag)
            }
        }
    }
}

fn get_random_number_request(battle_system: BattleSystem, stat: u8) -> RandomNumberRequest {
    match battle_system {
        BattleSystem::Original => RandomNumberRequest {
            numbers: 2,
            range: (0, 255),
        },
        BattleSystem::Dice { sides } => RandomNumberRequest {
            numbers: stat,
            range: (1, sides),
        },
        BattleSystem::Deterministic => RandomNumberRequest {
            numbers: 0,
            range: (0, 0),
        },
        BattleSystem::Test => RandomNumberRequest {
            numbers: 1,
            range: (0, 1),
        },
    }
}

fn does_interact(attacker: OwnedCard, defender: OwnedCard, arrow_to_defender: Arrows) -> bool {
    // they don't interact if both cards belong to the same player
    if defender.owner == attacker.owner {
        return false;
    }

    // they interact if the attacking card has an arrow in the direction of the defender
    attacker.card.arrows.has_any(arrow_to_defender)
}

// returns neighbouring cells along with the arrow that points at them
fn get_possible_neighbours(cell: u8) -> &'static [(u8, Arrows)] {
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

fn map_number_to_range(num: u8, range: impl std::ops::RangeBounds<u8>) -> u8 {
    // Simple way to map the given num to the range 0..max
    // This isn't a perfect mapping but will suffice
    // src: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
    fn map_0_to_max(num: u8, max: u8) -> u8 {
        ((num as u16 * max as u16) >> 8) as u8
    }

    use std::ops::Bound::*;

    let min = match range.start_bound() {
        Included(x) => *x,
        Excluded(x) => *x + 1,
        Unbounded => u8::MIN,
    };
    let max = match range.end_bound() {
        Included(x) => *x,
        Excluded(x) => *x - 1,
        Unbounded => u8::MAX,
    };
    debug_assert!(min <= max);

    if min == u8::MIN {
        if max == u8::MAX {
            num
        } else {
            map_0_to_max(num, max)
        }
    } else {
        min + map_0_to_max(num, max - min + 1)
    }
}
