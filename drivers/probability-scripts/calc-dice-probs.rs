#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! clap = { version = "4", features = ["derive"] }
//! serde_json="1"
//! ```

// based on: https://medium.com/swlh/dice-roll-distributions-statistics-and-the-importance-of-runtime-efficiency-d8ce3402db15

use clap::builder::TypedValueParser as _;
use clap::Parser;
use std::collections::HashMap;

#[derive(Parser)]
struct Args {
    #[arg(value_parser = clap::builder::PossibleValuesParser::new(["4", "6", "8", "10", "12"])
        .map(|s| s.parse::<u8>().unwrap()))]
    sides: u8,
}

fn main() {
    let sides = Args::parse().sides;

    let mut probs: HashMap<u8, HashMap<u8, f64>> = HashMap::with_capacity(0xF * 0xF);

    // map of every possible roll to number of times it comes up
    // for the {count} {sides} sided dice
    fn get_roll_counts(sides: u8, count: u8) -> Vec<(u8, u128)> {
        // mutates an existing slice of numbers to the next combination (with repetition)
        // returning false if there is no next combination
        fn next(max: u8, combi: &mut [u8]) -> bool {
            if combi[0] < max {
                combi[0] += 1;
                true
            } else if combi.len() > 1 {
                let res = next(max, &mut combi[1..]);
                combi[0] = combi[1];
                res
            } else {
                false
            }
        }

        let min_sum = count;
        let max_sum = sides * count;
        let capacity = (max_sum - min_sum + 1) as usize;
        let mut result = Vec::with_capacity(capacity);

        // pre-fill result with empty count for each possible sum
        for num in min_sum..=max_sum {
            result.push((num, 0));
        }

        // iterate over every combination
        let mut combi = vec![1; count as usize];
        loop {
            let sum: u8 = combi.iter().sum();
            let num_perms = num_unique_permutations(&combi);

            result[(sum - min_sum) as usize].1 += num_perms as u128;

            if !next(sides, &mut combi) {
                break;
            }
        }

        result
    }

    let mut all_roll_counts = Vec::with_capacity(16);
    all_roll_counts.push(vec![(0, 1)]);
    for value in 1..=0xF {
        all_roll_counts.push(get_roll_counts(sides, value));
    }

    for att in 0..=0xF {
        for def in 0..=0xF {
            eprint!("{att:X} v {def:X}");

            let mut total: u128 = 0;
            let mut att_wins: u128 = 0;
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

fn factorial(n: u8) -> usize {
    if n <= 1 {
        1
    } else {
        n as usize * factorial(n - 1)
    }
}

// calculate the number of unique permutations exist for the given slice of numbers
fn num_unique_permutations(nums: &[u8]) -> usize {
    let mut denominator = 1;
    let mut prev = nums[0];
    let mut run = 0;
    for &num in nums {
        if num != prev {
            denominator *= factorial(run);
            prev = num;
            run = 1;
        } else {
            run += 1;
        }
    }
    denominator *= factorial(run);

    factorial(nums.len() as u8) / denominator
}
