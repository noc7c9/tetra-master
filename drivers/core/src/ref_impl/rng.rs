use super::{
    Arrows, Board, Card, CardType, Cell, HandCandidate, HandCandidates, HAND_CANDIDATES, HAND_SIZE,
    MAX_NUMBER_OF_BLOCKS,
};
use rand::{distributions::uniform::SampleRange, thread_rng, Rng as _, SeedableRng as _};

type Seed = u64;

/// Wrapper around fastrand::Rng that keeps track of the initial seed
#[derive(Debug, Clone)]
pub(super) enum Rng {
    Internal {
        initial_seed: Seed,
        rng: rand_pcg::Pcg32,
    },
    External {
        random_numbers: std::collections::VecDeque<u8>,
    },
}

impl Rng {
    pub(super) fn new() -> Self {
        Self::with_seed(thread_rng().gen())
    }

    pub(super) fn with_seed(initial_seed: Seed) -> Self {
        Self::Internal {
            initial_seed,
            rng: rand_pcg::Pcg32::seed_from_u64(initial_seed),
        }
    }

    pub(super) fn new_external(random_numbers: std::collections::VecDeque<u8>) -> Self {
        Self::External { random_numbers }
    }

    pub(super) fn initial_seed(&self) -> Option<Seed> {
        match self {
            Self::Internal { initial_seed, .. } => Some(*initial_seed),
            Self::External { .. } => None,
        }
    }

    // generate methods

    pub(super) fn u8(&mut self, range: impl std::ops::RangeBounds<u8> + SampleRange<u8>) -> u8 {
        match self {
            Self::Internal { rng, .. } => rng.gen_range(range),
            Self::External { random_numbers } => {
                // Simple way to map the given num to the range 0..max
                // This isn't a perfect mapping but will suffice
                // src: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
                fn bound(num: u8, max: u8) -> u8 {
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

                let random_number = random_numbers
                    .pop_front()
                    .expect("Ran out of external random numbers");

                if min == u8::MIN {
                    if max == u8::MAX {
                        random_number
                    } else {
                        bound(random_number, max)
                    }
                } else {
                    min + bound(random_number, max - min + 1)
                }
            }
        }
    }
}

pub(super) fn random_board(rng: &mut Rng) -> Board {
    let mut board: Board = Default::default();

    // block cells
    for _ in 0..rng.u8(0..=MAX_NUMBER_OF_BLOCKS) {
        let idx = rng.u8(0..(HAND_SIZE as u8)) as usize;
        board[idx] = Cell::Blocked;
    }

    board
}

pub(super) fn random_hand_candidates(rng: &mut Rng) -> HandCandidates {
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
        .windows(HAND_CANDIDATES)
        .min_by(|a, b| {
            fn get_value_difference(hands: &[(f64, HandCandidate)]) -> f64 {
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

fn random_card(rng: &mut Rng) -> Card {
    fn randpick(rng: &mut Rng, values: &[u8]) -> u8 {
        let len = values.len();
        debug_assert!(len <= u8::MAX as usize);
        let idx = rng.u8(0..(len as u8)) as usize;
        values[idx]
    }

    fn random_stat(rng: &mut Rng) -> u8 {
        match rng.u8(0..=255) {
            0..=12 => randpick(rng, &[0, 1]),             // 5%
            13..=89 => randpick(rng, &[2, 3, 4, 5]),      // 30%
            90..=204 => randpick(rng, &[6, 7, 8, 9, 10]), // 45%
            205..=242 => randpick(rng, &[11, 12, 13]),    // 15%
            _ => randpick(rng, &[14, 15]),                // 5%
        }
    }

    let card_type = match rng.u8(0..=255) {
        0..=101 => CardType::Physical,  // 40%
        102..=203 => CardType::Magical, // 40%
        204..=241 => CardType::Exploit, // 15%
        _ => CardType::Assault,         // 5%
    };

    let arrows = Arrows(rng.u8(0..=u8::MAX));

    let attack = random_stat(rng);
    let physical_defense = random_stat(rng);
    let magical_defense = random_stat(rng);
    Card::new(attack, card_type, physical_defense, magical_defense, arrows)
}
