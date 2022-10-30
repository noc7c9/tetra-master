#!/usr/bin/env python3

# Arrows bit patterns
U = 0b1000_0000
UR = 0b0100_0000
R = 0b0010_0000
DR = 0b0001_0000
D = 0b0000_1000
DL = 0b0000_0100
L = 0b0000_0010
UL = 0b0000_0001

NEIGHBOURS = [
    [(0x1, R), (0x4, D), (0x5, DR)],
    [(0x0, L), (0x2, R), (0x4, DL), (0x5, D), (0x6, DR)],
    [(0x1, L), (0x3, R), (0x5, DL), (0x6, D), (0x7, DR)],
    [(0x2, L), (0x6, DL), (0x7, D)],
    [(0x0, U), (0x1, UR), (0x5, R), (0x8, D), (0x9, DR)],
    [(0x0, UL), (0x1, U), (0x2, UR), (0x4, L), (0x6, R), (0x8, DL), (0x9, D), (0xA, DR)],
    [(0x1, UL), (0x2, U), (0x3, UR), (0x5, L), (0x7, R), (0x9, DL), (0xA, D), (0xB, DR)],
    [(0x3, U), (0xB, D), (0xA, DL), (0x6, L), (0x2, UL)],
    [(0x4, U), (0x5, UR), (0x9, R), (0xD, DR), (0xC, D)],
    [(0x5, U), (0x6, UR), (0xA, R), (0xE, DR), (0xD, D), (0xC, DL), (0x8, L), (0x4, UL)],
    [(0x6, U), (0x7, UR), (0xB, R), (0xF, DR), (0xE, D), (0xD, DL), (0x9, L), (0x5, UL)],
    [(0x6, UL), (0x7, U), (0xA, L), (0xE, DL), (0xF, D)],
    [(0x8, U), (0x9, UR), (0xD, R)],
    [(0x8, UL), (0x9, U), (0xA, UR), (0xC, L), (0xE, R)],
    [(0x9, UL), (0xA, U), (0xB, UR), (0xD, L), (0xF, R)],
    [(0xA, UL), (0xB, U), (0xE, L)],
]

print("""// DO NOT MODIFY
// Generated using ./drivers/ai/generate-interactions.py

use tetra_master_core as core;

pub(crate) fn lookup(arrows: core::Arrows, cell: u8) -> u16 {
    INTERACTIONS[arrows.0 as usize][cell as usize]
}
""")

print("const INTERACTIONS: [[u16; 16]; 256] = [")
for arrows in range(0, 256):
    print(f"// Arrows: {arrows >> 4:04b}_{arrows & 0b1111:04b}")
    print("[")
    arrow_interactions = {}
    for cell in range(0, 16):
        interactions = 0
        for (neighbour_cell, arrow_back) in NEIGHBOURS[cell]:
            if (arrows & arrow_back) != 0:
                interactions = interactions ^ (1 << neighbour_cell)
        print("0b{:04b}_{:04b}_{:04b}_{:04b},".format(
            (interactions >> 12) & 0b1111,
            (interactions >> 8) & 0b1111,
            (interactions >> 4) & 0b1111,
            (interactions >> 0) & 0b1111,
        ))
    print("],")
print("];")
