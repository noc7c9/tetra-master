use tetra_master_core as core;

mod battle_system_probabilities;

pub mod naive_minimax;
pub mod random;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PlaceCard(core::PlaceCard),
    PickBattle(core::PickBattle),
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

    use super::naive_minimax;
    use super::{Action, Ai};
    use tetra_master_core::{
        self as core, Arrows, Card,
        Player::{Blue, Red},
    };

    const DEFAULT_DEPTH: usize = 3;
    const CARD: Card = Card::physical(0, 0, 0, Arrows(0));

    // TODO: change these to test-cases over each of the AIs

    #[test]
    fn sanity_check_1_one_move_left_need_flip_to_win() {
        let left = Card::physical(0, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, CARD];
        let hand_red = [CARD, CARD, CARD, CARD, left];
        let mut state = naive_minimax::init(
            DEFAULT_DEPTH,
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

    #[test]
    fn sanity_check_2_one_move_left_need_combo_to_win() {
        let def = Card::physical(0, 0, 0, Arrows::LEFT | Arrows::RIGHT);
        let att = Card::physical(0xF, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, def];
        let hand_red = [CARD, CARD, CARD, CARD, att];
        let mut state = naive_minimax::init(
            DEFAULT_DEPTH,
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

    #[test]
    fn sanity_check_3_one_move_left_pick_specific_battle_first() {
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
        let mut state = naive_minimax::init(
            DEFAULT_DEPTH,
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

    #[test]
    fn sanity_check_4_two_moves_left() {
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
        let mut state = naive_minimax::init(
            DEFAULT_DEPTH,
            core::Player::Red,
            &core::Setup {
                blocked_cells: [1, 10, 11].into(),
                hand_blue,
                hand_red,
                battle_system: core::BattleSystem::Deterministic,
                starting_player: Blue,
            },
        );

        let apply_place_card = |state: &mut naive_minimax::Ai, card, cell, player| {
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

    #[test]
    fn sanity_check_5_should_pick_obvious_great_move() {
        let mut driver = core::Driver::reference().seed(4763088336469180526).build();
        let setup = driver.random_setup(core::BattleSystem::Deterministic);
        let mut state = naive_minimax::init(3, core::Player::Red, &setup);

        let apply_place_card = |state: &mut naive_minimax::Ai, card, cell, player| {
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

    #[test]
    fn sanity_check_6_should_not_pick_obviously_bad_move() {
        let mut driver = core::Driver::reference().seed(4015497306351127204).build();
        let setup = driver.random_setup(core::BattleSystem::Deterministic);
        let mut state = naive_minimax::init(3, core::Player::Red, &setup);

        let apply_place_card = |state: &mut naive_minimax::Ai, card, cell, player| {
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
}
