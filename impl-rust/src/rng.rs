use crate::{Arrows, Board, Card, CardType, Cell, Hand, HAND_CANDIDATES, HAND_SIZE};

pub(crate) type Rng = fastrand::Rng;

pub(crate) fn random_seed() -> u64 {
    fastrand::u64(..)
}

pub(crate) fn random_board(rng: &Rng) -> Board {
    const MAX_NUMBER_OF_BLOCKS: u8 = 6;

    let mut board: Board = Default::default();

    // block cells
    for _ in 0..rng.u8(..=MAX_NUMBER_OF_BLOCKS) {
        let idx = rng.usize(..HAND_SIZE);
        board[idx] = Cell::Blocked;
    }

    board
}

pub(crate) fn random_hand_candidates(rng: &Rng) -> [Hand; HAND_CANDIDATES] {
    fn estimate_card_value(card: &Card) -> f64 {
        // very simple, we *don't* want this to be super accurate to allow the player to
        // strategize

        // value based on stats
        let stat_total = f64::from(card.attack)
            + f64::from(card.physical_defense)
            + f64::from(card.magical_defense);
        let stat_value = match card.card_type {
            CardType::Physical | CardType::Magical => 1.,
            CardType::Exploit => 1.75,
            CardType::Assault => 3.25,
        } * stat_total;

        // value based on arrows
        let num_arrows = {
            let mut arrows = card.arrows.0;
            let mut sum = 0;
            while arrows > 0 {
                if arrows & 0x1 == 1 {
                    sum += 1
                }
                arrows >>= 1;
            }
            sum
        };
        let arrows_value = match num_arrows {
            0 => 0.,
            1 | 8 => 2.,
            2 | 7 => 3.,
            3 | 6 => 4.,
            4 | 5 => 5.,
            _ => unreachable!(),
        };

        stat_value + arrows_value
    }

    // generate several random hands
    const INITIAL_SET: usize = 1000;
    let mut hands = Vec::with_capacity(INITIAL_SET);
    for _ in 0..INITIAL_SET {
        let hand = [
            Some(random_card(rng)),
            Some(random_card(rng)),
            Some(random_card(rng)),
            Some(random_card(rng)),
            Some(random_card(rng)),
        ];
        let value: f64 = hand
            .iter()
            .map(|card| estimate_card_value(&card.expect("card should exist")))
            .sum();
        hands.push((value, hand));
    }

    // find the hands with the most similar values
    hands.sort_unstable_by(|(a, _), (b, _)| a.total_cmp(b));
    let pick = hands
        .windows(HAND_CANDIDATES)
        .min_by(|a, b| {
            fn get_value_difference(hands: &[(f64, Hand)]) -> f64 {
                let (first, _) = hands.first().expect("window should not be empty");
                let (last, _) = hands.last().expect("window should not be empty");
                last - first
            }
            let a = get_value_difference(a);
            let b = get_value_difference(b);
            a.total_cmp(&b)
        })
        .expect("hands should not be empty");

    let candidates: Vec<_> = pick.iter().map(|&(_, hand)| hand).collect();
    candidates
        .try_into()
        .expect("pick should have correct length")
}

pub(crate) fn random_card(rng: &Rng) -> Card {
    fn randpick<'a, T>(rng: &Rng, values: &'a [T]) -> &'a T {
        let len = values.len();
        let idx = rng.usize(..len);
        &values[idx]
    }

    fn random_stat(rng: &Rng) -> u8 {
        let base_stat = *match rng.f32() {
            n if n < 0.05 => randpick(rng, &[0, 1]),          // 5%
            n if n < 0.35 => randpick(rng, &[2, 3, 4, 5]),    // 30%
            n if n < 0.8 => randpick(rng, &[6, 7, 8, 9, 10]), // 45%
            n if n < 0.95 => randpick(rng, &[11, 12, 13]),    // 15%
            _ => randpick(rng, &[14, 15]),                    // 5%
        };
        // base stats range from 0x0 to 0xF
        // real stats range from 0x0 to 0xFF
        0x10 * base_stat + rng.u8(..16)
    }

    let card_type = match rng.f32() {
        n if n < 0.40 => CardType::Physical, // 40%
        n if n < 0.80 => CardType::Magical,  // 40%
        n if n < 0.95 => CardType::Exploit,  // 15%
        _ => CardType::Assault,              // 5%
    };

    let arrows = Arrows(rng.u8(..));

    Card {
        card_type,
        arrows,
        attack: random_stat(rng),
        physical_defense: random_stat(rng),
        magical_defense: random_stat(rng),
    }
}
