use crate::{BattleResult, OwnedCard, Player, Seed};

#[derive(Debug, PartialEq)]
pub(crate) enum Entry {
    PreGameSetup {
        seed: Seed,
        p1_pick: usize,
        p2_pick: usize,
    },
    NextTurn {
        turn: Player,
    },
    PlaceCard {
        card: OwnedCard,
        cell: usize,
    },
    FlipCard {
        card: OwnedCard,
        cell: usize,
        to: Player,
        via_combo: bool,
    },
    Battle {
        attacker: OwnedCard,
        defender: OwnedCard,
        result: BattleResult,
    },
}

impl Entry {
    pub(crate) fn pre_game_setup(seed: Seed, p1_pick: usize, p2_pick: usize) -> Self {
        Entry::PreGameSetup {
            seed,
            p1_pick,
            p2_pick,
        }
    }

    pub(crate) fn next_turn(turn: Player) -> Self {
        Entry::NextTurn { turn }
    }

    pub(crate) fn place_card(card: OwnedCard, cell: usize) -> Self {
        Entry::PlaceCard { card, cell }
    }

    pub(crate) fn flip_card(card: OwnedCard, cell: usize, to: Player, via_combo: bool) -> Self {
        Entry::FlipCard {
            card,
            cell,
            to,
            via_combo,
        }
    }

    pub(crate) fn battle(attacker: OwnedCard, defender: OwnedCard, result: BattleResult) -> Self {
        Entry::Battle {
            attacker,
            defender,
            result,
        }
    }
}

pub(crate) struct GameLog {
    entries: Vec<Entry>,
}

impl GameLog {
    pub(crate) fn new() -> Self {
        GameLog { entries: vec![] }
    }

    pub(crate) fn append(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}
