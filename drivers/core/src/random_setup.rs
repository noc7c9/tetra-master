use super::{driver, Arrows, BoardCells, Card, CardType, Hand, BOARD_SIZE, MAX_NUMBER_OF_BLOCKS};
use rand::Rng as _;

pub(super) fn random_blocked_cells(rng: &mut driver::Rng) -> BoardCells {
    let mut blocked_cells = BoardCells::NONE;
    for _ in 0..rng.gen_range(0..=MAX_NUMBER_OF_BLOCKS) {
        loop {
            let cell = rng.gen_range(0..(BOARD_SIZE as u8));
            if !blocked_cells.has(cell) {
                blocked_cells.set(cell);
                break;
            }
        }
    }
    blocked_cells
}

pub(super) fn random_hands(rng: &mut driver::Rng) -> [Hand; 2] {
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
            random_card(rng),
            random_card(rng),
            random_card(rng),
            random_card(rng),
            random_card(rng),
        ];
        let value: f64 = hand.iter().map(estimate_card_value).sum();
        hands.push((value, hand));
    }

    // find the hands with the most similar values
    hands.sort_unstable_by(|(a, _), (b, _)| a.total_cmp(b));
    let pick = hands
        .windows(2)
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

fn random_card(rng: &mut driver::Rng) -> Card {
    fn randpick(rng: &mut driver::Rng, values: &[u8]) -> u8 {
        let len = values.len();
        debug_assert!(len <= u8::MAX as usize);
        let idx = rng.gen_range(0..(len as u8)) as usize;
        values[idx]
    }

    fn random_stat(rng: &mut driver::Rng) -> u8 {
        match rng.gen() {
            0u8..=12 => randpick(rng, &[0, 1]),           // 5%
            13..=89 => randpick(rng, &[2, 3, 4, 5]),      // 30%
            90..=204 => randpick(rng, &[6, 7, 8, 9, 10]), // 45%
            205..=242 => randpick(rng, &[11, 12, 13]),    // 15%
            _ => randpick(rng, &[14, 15]),                // 5%
        }
    }

    let card_type = match rng.gen() {
        0u8..=101 => CardType::Physical, // 40%
        102..=203 => CardType::Magical,  // 40%
        204..=241 => CardType::Exploit,  // 15%
        _ => CardType::Assault,          // 5%
    };

    let arrows = Arrows(rng.gen());

    let attack = random_stat(rng);
    let physical_defense = random_stat(rng);
    let magical_defense = random_stat(rng);
    Card::new(attack, card_type, physical_defense, magical_defense, arrows)
}
