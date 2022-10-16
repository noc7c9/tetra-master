#!/usr/bin/env python3

import json

def print_lookup_table(name, table):
    print()
    print(f"const {name}: [f32; 256] = [")
    for at in range(0, 0xF + 1):
        print(f"// Attack: {at:X}")
        for de in range(0, 0xF + 1):
            prob = float(table[str(at)][str(de)])
            print(f"{prob:f}, // {at:X} v {de:X}")
    print("];")


prob_deterministic = json.load(open("./data/deterministic.json"))
prob_original = json.load(open("./data/original.json"))
prob_dice_4 = json.load(open("./data/dice-4.json"))
prob_dice_6 = json.load(open("./data/dice-6.json"))
prob_dice_8 = json.load(open("./data/dice-8.json"))
prob_dice_10 = json.load(open("./data/dice-10.json"))
prob_dice_12 = json.load(open("./data/dice-12.json"))

print("""use tetra_master_core as core;

pub(crate) fn lookup(battle_system: core::BattleSystem, att: u8, def: u8) -> f32 {
    let key = att << 4 | def;
    let table = match battle_system {
        core::BattleSystem::Deterministic => PROBS_DETERMINISTIC,
        core::BattleSystem::Original => PROBS_ORIGINAL,
        core::BattleSystem::Dice { sides } => match sides {
            4 => PROBS_DICE_4,
            6 => PROBS_DICE_6,
            8 => PROBS_DICE_8,
            10 => PROBS_DICE_10,
            12 => PROBS_DICE_12,
            _ => panic!("unsupported"),
        },
        core::BattleSystem::Test => panic!("unsupported"),
    };
    table[key as usize]
}
""")

# write deterministic probabilities
print_lookup_table("PROBS_DETERMINISTIC", prob_deterministic)

# write original probabilities
print_lookup_table("PROBS_ORIGINAL", prob_original)

# write dice probabilities
print_lookup_table("PROBS_DICE_4", prob_dice_4)
print_lookup_table("PROBS_DICE_6", prob_dice_6)
print_lookup_table("PROBS_DICE_8", prob_dice_8)
print_lookup_table("PROBS_DICE_10", prob_dice_10)
print_lookup_table("PROBS_DICE_12", prob_dice_12)
