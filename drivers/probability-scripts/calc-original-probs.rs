#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::collections::HashMap;

fn main() {
    let mut probs: HashMap<u8, HashMap<u8, f64>> = HashMap::with_capacity(0xF * 0xF);

    // map of every possible rolled value to number of times it comes up
    // for the given stat value
    fn get_roll_counts(value: u8) -> HashMap<u8, u64> {
        let mut roll_counts = HashMap::new();
        for num_1 in 0..=255 {
            for num_2 in 0..=255 {
                let min = value << 4; // range: 00, 10, 20, ..., F0
                let max = min | 0xF; // range: 0F, 1F, 2F, ..., FF
                let stat1 = map_number_to_range(num_1, min..=max);
                let stat2 = map_number_to_range(num_2, ..=stat1);
                let roll = stat1 - stat2;
                *roll_counts.entry(roll).or_default() += 1;
            }
        }
        roll_counts
    }

    let mut all_roll_counts = Vec::with_capacity(16);
    for value in 0..=0xF {
        all_roll_counts.push(get_roll_counts(value));
    }

    for att in 0..=0xF {
        for def in 0..=0xF {
            eprint!("{att:X} v {def:X}");

            let mut total: u64 = 0;
            let mut att_wins: u64 = 0;
            for (att_roll, att_count) in &all_roll_counts[att as usize] {
                for (def_roll, def_count) in &all_roll_counts[def as usize] {
                    total += att_count * def_count;
                    if att_roll > def_roll {
                        att_wins += att_count * def_count;
                    }
                }
            }
            let prob = att_wins as f64 / total as f64;
            probs.entry(att).or_default().insert(def, prob);

            eprintln!(": {:?}", (att_wins, prob));
        }
    }

    serde_json::to_writer(std::io::stdout(), &probs).unwrap();
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
