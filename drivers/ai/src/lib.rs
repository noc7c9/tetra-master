use tetra_master_core as core;

mod battle_system_probabilities;

#[cfg(feature = "logging")]
#[macro_use]
mod logging;

#[cfg(not(feature = "logging"))]
#[macro_use]
mod logging_noop;

mod metrics;

pub mod naive_minimax;
pub mod random;

pub mod expectiminimax_0_naive;
pub mod expectiminimax_1_simplify;
pub mod expectiminimax_2_ab_pruning;
pub mod expectiminimax_3_negamax;
pub mod expectiminimax_4_prob_cutoff;
pub mod expectiminimax_5_no_alloc_get_resolutions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PlaceCard(core::PlaceCard),
    PickBattle(core::PickBattle),
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let player = match match self {
            Action::PlaceCard(core::PlaceCard { player, .. }) => player,
            Action::PickBattle(core::PickBattle { player, .. }) => player,
        } {
            core::Player::Red => "Red ",
            core::Player::Blue => "Blue",
        };
        match self {
            Action::PlaceCard(core::PlaceCard { card, cell, .. }) => {
                write!(f, "{player} Place({card:X}, {cell:X})")
            }
            Action::PickBattle(core::PickBattle { cell, .. }) => {
                write!(f, "{player} Pick({cell:X})    ")
            }
        }
    }
}

pub trait Ai {
    fn get_action(&mut self) -> Action;

    fn apply_place_card(&mut self, cmd: core::PlaceCard);
    fn apply_pick_battle(&mut self, cmd: core::PickBattle);
    fn apply_resolve_battle(&mut self, cmd: &core::ResolveBattle);

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::PlaceCard(cmd) => self.apply_place_card(cmd),
            Action::PickBattle(cmd) => self.apply_pick_battle(cmd),
        }
    }
}

#[cfg(test)]
mod tests {
    //  0 | 1 | 2 | 3
    // ---+---+---+---
    //  4 | 5 | 6 | 7
    // ---+---+---+---
    //  8 | 9 | A | B
    // ---+---+---+---
    //  C | D | E | F

    use test_case::test_case;
    use tetra_master_core::{
        self as core, Arrows, Card,
        Player::{Blue, Red},
    };

    use super::{Action, Ai};

    type Initializer = Box<dyn Fn(core::Player, &core::Setup) -> Box<dyn Ai>>;

    const CARD: Card = Card::physical(0, 0, 0, Arrows(0));

    macro_rules! init {
        ($name:ident) => { init!($name,) };
        ($name:ident, $($arg:expr),* $(,)?) => {{
            Box::new(|player, cmd| Box::new(crate::$name::init($($arg,)* player, cmd)))
        }};
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_1_one_move_left_need_flip_to_win(init: Initializer) {
        let left = Card::physical(0, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, CARD];
        let hand_red = [CARD, CARD, CARD, CARD, left];
        let mut state = init(
            core::Player::Red,
            &core::Setup {
                blocked_cells: core::BoardCells::NONE,
                hand_blue,
                hand_red,
                battle_system: core::BattleSystem::Deterministic,
                starting_player: Blue,
            },
        );

        let mut apply_place_card = |card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(0, 0, Blue);
        apply_place_card(0, 1, Red);

        apply_place_card(1, 2, Blue);
        apply_place_card(1, 3, Red);

        apply_place_card(2, 4, Blue);
        apply_place_card(2, 5, Red);

        apply_place_card(3, 6, Blue);
        apply_place_card(3, 7, Red);

        apply_place_card(4, 9, Blue);

        //  b | r | b | r
        // ---+---+---+---
        //  b | r | b | r
        // ---+---+---+---
        //  _ | b | X | _
        // ---+---+---+---
        //  _ | _ | _ | _
        //
        // Expected Action: Card 4 on X
        //  which will flip the blue card on 9
        //  resulting in a score of 4 v 6

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 4,
            cell: 0xA,
            player: Red,
        });
        assert_eq!(actual, expected);
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_2_one_move_left_need_combo_to_win(init: Initializer) {
        let def = Card::physical(0, 0, 0, Arrows::LEFT | Arrows::RIGHT);
        let att = Card::physical(0xF, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, def];
        let hand_red = [CARD, CARD, CARD, CARD, att];
        let mut state = init(
            core::Player::Red,
            &core::Setup {
                blocked_cells: core::BoardCells::NONE,
                hand_blue,
                hand_red,
                battle_system: core::BattleSystem::Deterministic,
                starting_player: Blue,
            },
        );

        let mut apply_place_card = |card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(0, 3, Blue);
        apply_place_card(0, 0, Red);

        apply_place_card(1, 2, Blue);
        apply_place_card(1, 1, Red);

        apply_place_card(2, 7, Blue);
        apply_place_card(2, 4, Red);

        apply_place_card(3, 8, Blue);
        apply_place_card(3, 0xB, Red);

        apply_place_card(4, 5, Blue);

        //  r | r | b | b
        // ---+---+---+---
        //  b |<b>| X | b
        // ---+---+---+---
        //  b | _ | _ | r
        // ---+---+---+---
        //  _ | _ | _ | _
        //
        // Expected Action: Card 4 on X
        //  which will attack (and flip) the blue card on 5
        //  which in turn combo flips the blue card on 4
        //  resulting in a score of 4 v 6

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 4,
            cell: 6,
            player: Red,
        });
        assert_eq!(actual, expected);
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_3_one_move_left_pick_specific_battle_first(init: Initializer) {
        let hand_blue = [
            CARD,
            CARD,
            CARD,
            Card::physical(0, 0, 0, Arrows::DOWN | Arrows::DOWN_RIGHT),
            Card::physical(
                0,
                0,
                0,
                Arrows::LEFT | Arrows::UP_LEFT | Arrows::UP | Arrows::UP_RIGHT | Arrows::RIGHT,
            ),
        ];
        let hand_red = [
            CARD,
            CARD,
            CARD,
            CARD,
            Card::physical(1, 0, 0, Arrows::UP | Arrows::UP_RIGHT | Arrows::RIGHT),
        ];
        let mut state = init(
            core::Player::Red,
            &core::Setup {
                blocked_cells: core::BoardCells::NONE,
                hand_blue,
                hand_red,
                battle_system: core::BattleSystem::Deterministic,
                starting_player: Blue,
            },
        );

        let mut apply_place_card = |card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(0, 0, Blue);
        apply_place_card(0, 3, Red);

        apply_place_card(1, 2, Blue);
        apply_place_card(1, 7, Red);

        apply_place_card(2, 1, Blue);
        apply_place_card(2, 6, Red);

        apply_place_card(3, 5, Blue);
        apply_place_card(3, 11, Red);

        apply_place_card(4, 10, Blue);

        //  b | b | b | r
        // ---+---+---+---
        //  _ | b | b | b
        // ---+-v-\-^-/---
        //  _ | X < b > b
        // ---+---+---+---
        //  _ | _ | _ | #
        //
        // Expected Action: Card 4 on X and picking to battle blue card on 10 first
        //  which will flip the attacked card and combo flip the 4 cards around
        //  resulting in a score of 3 v 7

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 4,
            cell: 9,
            player: Red,
        });
        assert_eq!(actual, expected);

        state.apply_action(expected);

        let actual = state.get_action();
        let expected = Action::PickBattle(core::PickBattle {
            cell: 10,
            player: Red,
        });
        assert_eq!(actual, expected);
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_4_two_moves_left(init: Initializer) {
        let hand_blue = [
            CARD,
            CARD,
            Card::physical(0, 0, 0xF, Arrows::LEFT),
            Card::physical(
                0,
                0xF,
                0,
                Arrows::LEFT | Arrows::UP | Arrows::UP_RIGHT | Arrows::RIGHT,
            ),
            Card::physical(0, 0, 0, Arrows::RIGHT),
        ];
        let hand_red = [
            CARD,
            CARD,
            CARD,
            Card::physical(0xF, 0, 0, Arrows::RIGHT),
            Card::magical(0xF, 0, 0, Arrows::RIGHT),
        ];
        let mut state = init(
            core::Player::Red,
            &core::Setup {
                blocked_cells: [1, 10, 11].into(),
                hand_blue,
                hand_red,
                battle_system: core::BattleSystem::Deterministic,
                starting_player: Blue,
            },
        );

        let apply_place_card = |state: &mut Box<dyn Ai>, card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(&mut state, 0, 0, Blue);
        apply_place_card(&mut state, 0, 2, Red);

        apply_place_card(&mut state, 1, 0xC, Blue);
        apply_place_card(&mut state, 1, 3, Red);

        apply_place_card(&mut state, 2, 0xF, Blue);
        apply_place_card(&mut state, 2, 7, Red);

        apply_place_card(&mut state, 3, 6, Blue);

        //  b | # | b | b
        // ---+---+-^-/---
        //  _ | X < b > b
        // ---+---+---+---
        //  _ | _ | # | #
        // ---+---+---+---
        //  b | _ | _ < b
        //
        // Expected Action: Card 4 (magical) on X
        //  which will attack (and flip) the blue card on 6
        //  which in turn combo flips the cards on 3, 4 and 7
        //  resulting in a score of 3 v 5

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 4,
            cell: 5,
            player: Red,
        });
        assert_eq!(actual, expected);

        apply_place_card(&mut state, 4, 5, Red); // AI move
        state.apply_resolve_battle(&core::ResolveBattle {
            attack_roll: vec![],
            defend_roll: vec![],
        });
        apply_place_card(&mut state, 4, 4, Blue); // response

        //  b | # | r | r
        // ---+---+-^-/---
        //  b | b < r > r
        // ---+---+---+---
        //  _ | _ | # | #
        // ---+---+---+---
        //  b | _ | X < b
        //
        // Expected Action: Card 3 on X
        //  which will attack (and flip) the blue card on F
        //  resulting in a score of 4 v 6

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 3,
            cell: 0xE,
            player: Red,
        });
        assert_eq!(actual, expected);
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_5_should_pick_obvious_great_move(init: Initializer) {
        let mut driver = core::Driver::reference().seed(4763088336469180526).build();
        let setup = driver.random_setup(core::BattleSystem::Deterministic);
        let mut state = init(core::Player::Red, &setup);

        let apply_place_card = |state: &mut Box<dyn Ai>, card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(&mut state, 2, 0xF, Blue);

        let actual = state.get_action();
        let expected = Action::PlaceCard(core::PlaceCard {
            card: 1,
            cell: 14,
            player: Red,
        });
        assert_eq!(actual, expected);
    }

    #[test_case(init!(naive_minimax, 3))]
    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_6_should_not_pick_obviously_bad_move(init: Initializer) {
        let mut driver = core::Driver::reference().seed(4015497306351127204).build();
        let setup = driver.random_setup(core::BattleSystem::Deterministic);
        let mut state = init(core::Player::Red, &setup);

        let apply_place_card = |state: &mut Box<dyn Ai>, card, cell, player| {
            let cmd = core::PlaceCard { player, card, cell };
            state.apply_place_card(cmd);
        };

        apply_place_card(&mut state, 4, 1, Blue);

        let actual = state.get_action();
        let not_expected = Action::PlaceCard(core::PlaceCard {
            card: 0,
            cell: 2,
            player: Red,
        });
        assert_ne!(actual, not_expected);
    }

    #[test_case(init!(expectiminimax_0_naive, 3))]
    #[test_case(init!(expectiminimax_1_simplify, 3))]
    #[test_case(init!(expectiminimax_2_ab_pruning, 3))]
    #[test_case(init!(expectiminimax_3_negamax, 3))]
    #[test_case(init!(expectiminimax_4_prob_cutoff, 3, 0.0))]
    #[test_case(init!(expectiminimax_5_no_alloc_get_resolutions, 3, 0.0))]
    fn sanity_check_7_pick_more_likely_battle_when_using_non_deterministic_battle_systems(
        init: Initializer,
    ) {
        for battle_system in [
            core::BattleSystem::Original,
            core::BattleSystem::Dice { sides: 4 },
            core::BattleSystem::Dice { sides: 6 },
            core::BattleSystem::Dice { sides: 8 },
            core::BattleSystem::Dice { sides: 10 },
            core::BattleSystem::Dice { sides: 12 },
        ] {
            let hand_blue = [
                CARD,
                CARD,
                CARD,
                Card::physical(0, 9, 0, Arrows::DOWN | Arrows::DOWN_RIGHT),
                Card::physical(0, 7, 0, Arrows::LEFT | Arrows::UP_LEFT),
            ];
            let hand_red = [
                CARD,
                CARD,
                CARD,
                CARD,
                Card::physical(3, 0, 0, Arrows::UP | Arrows::RIGHT),
            ];
            let mut state = init(
                core::Player::Red,
                &core::Setup {
                    blocked_cells: core::BoardCells::NONE,
                    hand_blue,
                    hand_red,
                    battle_system,
                    starting_player: Blue,
                },
            );

            let apply_place_card = |state: &mut Box<dyn Ai>, card, cell, player| {
                let cmd = core::PlaceCard { player, card, cell };
                state.apply_place_card(cmd);
            };

            apply_place_card(&mut state, 0, 0, Blue);
            apply_place_card(&mut state, 0, 1, Red);

            apply_place_card(&mut state, 1, 2, Blue);
            apply_place_card(&mut state, 1, 3, Red);

            apply_place_card(&mut state, 2, 4, Blue);
            apply_place_card(&mut state, 2, 5, Red);

            apply_place_card(&mut state, 3, 6, Blue); // points down at 10 and at 11
            apply_place_card(&mut state, 3, 7, Red);

            apply_place_card(&mut state, 4, 11, Blue); // points left at 10 and at 6
            apply_place_card(&mut state, 4, 10, Red); // attacks 6 and 11

            //  b | r | b | r
            // ---+---+---+---
            //  b | r | b | r
            // ---+---+-v-\---
            //  _ | _ | r < b
            // ---+---+---+---
            //  _ | _ | _ | _
            //
            // Expected Action:
            //  Picking to battle card on 11 which has a defense of 7
            //  and NOT the card on 6 which has a defense of 9.
            //  As both cards point at each other winning either battle would skip the other battle,
            //  so by defeating the weaker card, the stronger card won't have to be fought

            let actual = state.get_action();
            let expected = Action::PickBattle(core::PickBattle {
                cell: 11,
                player: Red,
            });
            assert_eq!(actual, expected);
        }
    }
}
