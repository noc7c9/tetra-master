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
        attacker_cell: usize,
        defender: OwnedCard,
        defender_cell: usize,
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

    pub(crate) fn battle(
        attacker: OwnedCard,
        attacker_cell: usize,
        defender: OwnedCard,
        defender_cell: usize,
        result: BattleResult,
    ) -> Self {
        Entry::Battle {
            attacker,
            attacker_cell,
            defender,
            defender_cell,
            result,
        }
    }
}

pub(crate) struct GameLog {
    entries: Vec<Entry>,
    last_new_entries_idx: usize,
}

impl GameLog {
    pub(crate) fn new() -> Self {
        GameLog {
            entries: vec![],
            last_new_entries_idx: 0,
        }
    }

    pub(crate) fn append(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub(crate) fn new_entries(&mut self) -> &[Entry] {
        let idx = self.last_new_entries_idx;
        self.last_new_entries_idx = self.entries.len() - 1;
        &self.entries[idx..]
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}
