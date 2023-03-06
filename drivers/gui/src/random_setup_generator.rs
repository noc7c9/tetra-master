use tetra_master_core as core;

use super::common::{Card, Hand};

const TEMPLATES: [CardTemplate; 100] = {
    use tetra_master_core::CardType::{Magical as Mag, Physical as Phy};
    [
        CardTemplate::new(1, "Goblin", 0, Phy, 0, 0),
        CardTemplate::new(2, "Fang", 0, Phy, 0, 0),
        CardTemplate::new(3, "Skeleton", 0, Phy, 0, 0),
        CardTemplate::new(4, "Flan", 0, Mag, 0, 1),
        CardTemplate::new(5, "Zaghnol", 0, Phy, 0, 0),
        CardTemplate::new(6, "Lizard Man", 0, Phy, 0, 0),
        CardTemplate::new(7, "Zombie", 1, Mag, 1, 0),
        CardTemplate::new(8, "Bomb", 1, Mag, 0, 1),
        CardTemplate::new(9, "Ironite", 1, Phy, 1, 0),
        CardTemplate::new(10, "Sahagin", 1, Phy, 1, 0),
        CardTemplate::new(11, "Yeti", 1, Mag, 0, 1),
        CardTemplate::new(12, "Mimic", 1, Mag, 1, 1),
        CardTemplate::new(13, "Wyerd", 1, Mag, 0, 1),
        CardTemplate::new(14, "Mandragora", 1, Mag, 0, 2),
        CardTemplate::new(15, "Crawler", 2, Phy, 2, 0),
        CardTemplate::new(16, "Sand Scorpion", 2, Phy, 2, 0),
        CardTemplate::new(17, "Nymph", 2, Mag, 0, 2),
        CardTemplate::new(18, "Sand Golem", 2, Phy, 2, 0),
        CardTemplate::new(19, "Zuu", 2, Phy, 0, 2),
        CardTemplate::new(20, "Dragonfly", 2, Phy, 2, 1),
        CardTemplate::new(21, "Carrion Worm", 2, Mag, 1, 1),
        CardTemplate::new(22, "Cerberus", 2, Phy, 2, 0),
        CardTemplate::new(23, "Antlion", 3, Phy, 2, 1),
        CardTemplate::new(24, "Cactuar", 3, Phy, 0xC, 0),
        CardTemplate::new(25, "Gimme Cat", 3, Mag, 1, 1),
        CardTemplate::new(26, "Ragtimer", 3, Mag, 2, 1),
        CardTemplate::new(27, "Hedgehog Pie", 3, Mag, 1, 2),
        CardTemplate::new(28, "Ralvuimago", 3, Phy, 4, 0),
        CardTemplate::new(29, "Ochu", 3, Phy, 2, 1),
        CardTemplate::new(30, "Troll", 3, Phy, 3, 2),
        CardTemplate::new(31, "Blazer Beetle", 4, Phy, 5, 1),
        CardTemplate::new(32, "Abomination", 4, Phy, 3, 3),
        CardTemplate::new(33, "Zemzelett", 4, Mag, 1, 5),
        CardTemplate::new(34, "Stroper", 4, Phy, 3, 0),
        CardTemplate::new(35, "Tantarian", 4, Mag, 2, 2),
        CardTemplate::new(36, "Grand Dragon", 4, Phy, 4, 4),
        CardTemplate::new(37, "Feather Circle", 4, Mag, 2, 2),
        CardTemplate::new(38, "Hecteyes", 4, Mag, 0, 4),
        CardTemplate::new(39, "Ogre", 5, Phy, 4, 1),
        CardTemplate::new(40, "Armstrong", 5, Mag, 2, 4),
        CardTemplate::new(41, "Ash", 5, Mag, 3, 3),
        CardTemplate::new(42, "Wraith", 5, Mag, 4, 0),
        CardTemplate::new(43, "Gargoyle", 5, Mag, 3, 2),
        CardTemplate::new(44, "Vepal", 5, Mag, 3, 3),
        CardTemplate::new(45, "Grimlock", 5, Mag, 2, 3),
        CardTemplate::new(46, "Tonberry", 2, Phy, 3, 3),
        CardTemplate::new(47, "Veteran", 5, Mag, 1, 9),
        CardTemplate::new(48, "Garuda", 6, Mag, 4, 1),
        CardTemplate::new(49, "Malboro", 5, Mag, 3, 6),
        CardTemplate::new(50, "Mover", 6, Mag, 0xF, 0),
        CardTemplate::new(51, "Abadon", 7, Mag, 6, 2),
        CardTemplate::new(52, "Behemoth", 0xB, Phy, 4, 6),
        CardTemplate::new(53, "Iron Man", 0xC, Phy, 6, 0),
        CardTemplate::new(54, "Nova Dragon", 0xE, Phy, 7, 0xC),
        CardTemplate::new(55, "Ozma", 0xD, Mag, 0, 0xC),
        CardTemplate::new(56, "Hades", 0xF, Mag, 0xC, 1),
        CardTemplate::new(57, "Holy", 8, Mag, 2, 3),
        CardTemplate::new(58, "Meteor", 0xB, Mag, 0xA, 0),
        CardTemplate::new(59, "Flare", 0xC, Mag, 0, 0),
        CardTemplate::new(60, "Shiva", 5, Mag, 0, 5),
        CardTemplate::new(61, "Ifrit", 6, Mag, 9, 0),
        CardTemplate::new(62, "Ramuh", 4, Mag, 1, 6),
        CardTemplate::new(63, "Atomos", 4, Mag, 6, 6),
        CardTemplate::new(64, "Odin", 0xC, Mag, 8, 4),
        CardTemplate::new(65, "Leviathan", 0xB, Mag, 6, 1),
        CardTemplate::new(66, "Bahamut", 0xC, Mag, 8, 5),
        CardTemplate::new(67, "Ark", 0xE, Mag, 5, 5),
        CardTemplate::new(68, "Fenrir", 8, Mag, 2, 1),
        CardTemplate::new(69, "Madeen", 0xA, Mag, 1, 6),
        CardTemplate::new(70, "Alexander", 0xD, Mag, 0xB, 5),
        CardTemplate::new(71, "Excalibur II", 0xF, Phy, 0xB, 0),
        CardTemplate::new(72, "Ultima Weapon", 0xF, Phy, 1, 6),
        CardTemplate::new(73, "Masamune", 0xC, Phy, 0xB, 3),
        CardTemplate::new(74, "Elixir", 6, Mag, 6, 6),
        CardTemplate::new(75, "Dark Matter", 0xC, Mag, 3, 0xC),
        CardTemplate::new(76, "Ribbon", 0, Mag, 0xC, 0xF),
        CardTemplate::new(77, "Tiger Racket", 0, Phy, 0, 1),
        CardTemplate::new(78, "Save the Queen", 6, Phy, 3, 0),
        CardTemplate::new(79, "Genji", 0, Phy, 6, 0xA),
        CardTemplate::new(80, "Mythril Sword", 1, Phy, 0, 0),
        CardTemplate::new(81, "Blue Narciss", 8, Phy, 8, 1),
        CardTemplate::new(82, "Hilda Garde 3", 6, Phy, 3, 0),
        CardTemplate::new(83, "Invincible", 0xB, Phy, 8, 0xC),
        CardTemplate::new(84, "Cargo Ship", 2, Phy, 6, 0),
        CardTemplate::new(85, "Hilda Garde 1", 6, Phy, 4, 0),
        CardTemplate::new(86, "Red Rose", 8, Phy, 1, 8),
        CardTemplate::new(87, "Theater Ship", 1, Phy, 6, 1),
        CardTemplate::new(88, "Viltgance", 0xE, Phy, 8, 1),
        CardTemplate::new(89, "Chocobo", 0, Phy, 0, 0),
        CardTemplate::new(90, "Fat Chocobo", 1, Phy, 1, 1),
        CardTemplate::new(91, "Mog", 1, Mag, 0, 0),
        CardTemplate::new(92, "Frog", 0, Phy, 0, 0),
        CardTemplate::new(93, "Oglop", 2, Phy, 1, 0),
        CardTemplate::new(94, "Alexandria", 0, Phy, 0xB, 6),
        CardTemplate::new(95, "Lindblum", 0, Phy, 6, 0xB),
        CardTemplate::new(96, "Two Moons", 6, Mag, 5, 5),
        CardTemplate::new(97, "Gargant", 2, Phy, 0, 3),
        CardTemplate::new(98, "Namingway", 7, Mag, 7, 7),
        CardTemplate::new(99, "Boco", 7, Phy, 7, 7),
        CardTemplate::new(100, "Airship", 7, Phy, 7, 7),
    ]
};

pub struct CardTemplate {
    image_index: usize,
    name: &'static str,

    attack: u8,
    card_type: core::CardType,
    physical_defense: u8,
    magical_defense: u8,
}

impl CardTemplate {
    const fn new(
        image_index: usize,
        name: &'static str,
        attack: u8,
        card_type: core::CardType,
        physical_defense: u8,
        magical_defense: u8,
    ) -> Self {
        Self {
            image_index,
            name,

            attack,
            card_type,
            physical_defense,
            magical_defense,
        }
    }

    fn init(&self, arrows: core::Arrows) -> Card {
        let stats = core::Card::new(
            self.attack,
            self.card_type,
            self.physical_defense,
            self.magical_defense,
            arrows,
        );
        Card {
            image_index: self.image_index,
            name: self.name,
            stats,
        }
    }
}

pub fn random_card(rng: &mut core::Rng) -> Card {
    use once_cell::sync::Lazy;
    static NUM_ARROWS_WEIGHTS: Lazy<core::WeightedIndex<u8>> = Lazy::new(|| {
        core::Rng::weighted_index([
            // index is the number of arrows
            1,  // 0
            5,  // 1
            10, // 2
            15, // 3
            18, // 4
            13, // 5
            9,  // 6
            5,  // 7
            1,  // 8
        ])
    });

    let index = rng.gen_range(15..=35);

    let arrows = {
        let num_arrows = rng.sample(&*NUM_ARROWS_WEIGHTS);
        loop {
            let arrows: u8 = rng.gen();
            if arrows.count_ones() == num_arrows as u32 {
                break core::Arrows(arrows);
            }
        }
    };

    TEMPLATES[index].init(arrows)
}

pub fn random_hand(rng: &mut core::Rng) -> Hand {
    [
        random_card(rng),
        random_card(rng),
        random_card(rng),
        random_card(rng),
        random_card(rng),
    ]
}

pub fn random_starting_player(rng: &mut core::Rng) -> core::Player {
    if rng.gen() {
        core::Player::Blue
    } else {
        core::Player::Red
    }
}

pub fn random_blocked_cells(rng: &mut core::Rng) -> core::BoardCells {
    let mut blocked_cells = core::BoardCells::NONE;
    for _ in 0..rng.gen_range(0..=core::MAX_NUMBER_OF_BLOCKS) {
        loop {
            let cell = rng.gen_range(0..(core::BOARD_SIZE as u8));
            if !blocked_cells.has(cell) {
                blocked_cells.set(cell);
                break;
            }
        }
    }
    blocked_cells
}
