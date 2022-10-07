pub mod naive_minimax;

#[cfg(test)]
mod tests {
    //  0 | 1 | 2 | 3
    // ---+---+---+---
    //  4 | 5 | 6 | 7
    // ---+---+---+---
    //  8 | 9 | A | B
    // ---+---+---+---
    //  C | D | E | F

    use super::naive_minimax::{minimax_search, minimax_search_with_logging, Action, State};
    use tetra_master_core::{self as core, Arrows, Card};

    const CARD: Card = Card::physical(0, 0, 0, Arrows(0));

    #[test]
    fn sanity_check_1_move_left_need_flip_to_win() {
        let left = Card::physical(0, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, CARD];
        let hand_red = [CARD, CARD, CARD, CARD, left];
        let mut state = State::new(
            core::Player::P1,
            core::BoardCells::NONE,
            hand_blue,
            hand_red,
            core::BattleSystem::Deterministic,
        );

        let mut apply_place_card = |card, cell| {
            let cmd = core::command::PlaceCard { card, cell };
            state.apply_in_place(Action::PlaceCard(cmd));
        };

        apply_place_card(0, 0);
        apply_place_card(0, 1);

        apply_place_card(1, 2);
        apply_place_card(1, 3);

        apply_place_card(2, 4);
        apply_place_card(2, 5);

        apply_place_card(3, 6);
        apply_place_card(3, 7);

        apply_place_card(4, 9);

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

        let actual = minimax_search(state);
        let expected = Action::PlaceCard(core::command::PlaceCard { card: 4, cell: 0xA });
        assert_eq!(actual, expected);
    }

    #[test]
    fn sanity_check_1_move_left_need_combo_to_win() {
        let def = Card::physical(0, 0, 0, Arrows::LEFT | Arrows::RIGHT);
        let att = Card::physical(0xF, 0, 0, Arrows::LEFT);

        let hand_blue = [CARD, CARD, CARD, CARD, def];
        let hand_red = [CARD, CARD, CARD, CARD, att];
        let mut state = State::new(
            core::Player::P1,
            core::BoardCells::NONE,
            hand_blue,
            hand_red,
            core::BattleSystem::Deterministic,
        );

        let mut apply_place_card = |card, cell| {
            let cmd = core::command::PlaceCard { card, cell };
            state.apply_in_place(Action::PlaceCard(cmd));
        };

        apply_place_card(0, 3);
        apply_place_card(0, 0);

        apply_place_card(1, 2);
        apply_place_card(1, 1);

        apply_place_card(2, 7);
        apply_place_card(2, 4);

        apply_place_card(3, 8);
        apply_place_card(3, 0xB);

        apply_place_card(4, 5);

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

        let actual = minimax_search(state);
        let expected = Action::PlaceCard(core::command::PlaceCard { card: 4, cell: 6 });
        assert_eq!(actual, expected);
    }

    #[test]
    fn sanity_check_1_move_left_pick_specific_battle_first() {
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
        let mut state = State::new(
            core::Player::P1,
            core::BoardCells::NONE,
            hand_blue,
            hand_red,
            core::BattleSystem::Deterministic,
        );

        let mut apply_place_card = |card, cell| {
            let cmd = core::command::PlaceCard { card, cell };
            state.apply_in_place(Action::PlaceCard(cmd));
        };

        apply_place_card(0, 0);
        apply_place_card(0, 3);

        apply_place_card(1, 2);
        apply_place_card(1, 7);

        apply_place_card(2, 1);
        apply_place_card(2, 6);

        apply_place_card(3, 5);
        apply_place_card(3, 11);

        apply_place_card(4, 10);

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

        let actual = minimax_search_with_logging(state.clone());
        let expected = Action::PlaceCard(core::command::PlaceCard { card: 4, cell: 9 });
        assert_eq!(actual, expected);

        state.apply_in_place(expected);

        let actual = minimax_search(state);
        let expected = Action::PickBattle(core::command::PickBattle { cell: 10 });
        assert_eq!(actual, expected);
    }

    #[test]
    fn sanity_check_2_moves_left() {
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
        let mut state = State::new(
            core::Player::P1,
            [1, 10, 11].into(),
            hand_blue,
            hand_red,
            core::BattleSystem::Deterministic,
        );

        let apply_place_card = |state: &mut State, card, cell| {
            let cmd = core::command::PlaceCard { card, cell };
            state.apply_in_place(Action::PlaceCard(cmd));
        };

        apply_place_card(&mut state, 0, 0);
        apply_place_card(&mut state, 0, 2);

        apply_place_card(&mut state, 1, 0xC);
        apply_place_card(&mut state, 1, 3);

        apply_place_card(&mut state, 2, 0xF);
        apply_place_card(&mut state, 2, 7);

        apply_place_card(&mut state, 3, 6);

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

        let actual = minimax_search(state.clone());
        let expected = Action::PlaceCard(core::command::PlaceCard { card: 4, cell: 5 });
        assert_eq!(actual, expected);

        apply_place_card(&mut state, 4, 5); // AI move
        apply_place_card(&mut state, 4, 4); // response

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

        let actual = minimax_search(state);
        let expected = Action::PlaceCard(core::command::PlaceCard { card: 3, cell: 0xE });
        assert_eq!(actual, expected);
    }
}
