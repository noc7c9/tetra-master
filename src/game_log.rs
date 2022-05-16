use crate::{BattleResult, Card, Player};

#[derive(Debug, PartialEq)]
pub(crate) enum Entry {
    NextTurn {
        turn: Player,
    },
    PlaceCard {
        card: Card,
        cell: usize,
        owner: Player,
    },
    FlipCard {
        card: Card,
        cell: usize,
        to: Player,
    },
    Battle {
        attacker: (Player, Card),
        defender: (Player, Card),
        result: BattleResult,
    },
}

impl Entry {
    pub(crate) fn next_turn(turn: Player) -> Self {
        Entry::NextTurn { turn }
    }

    pub(crate) fn place_card(card: &Card, cell: usize, owner: Player) -> Self {
        let card = card.clone();
        Entry::PlaceCard { card, cell, owner }
    }

    pub(crate) fn flip_card(card: &Card, cell: usize, to: Player) -> Self {
        let card = card.clone();
        Entry::FlipCard { card, cell, to }
    }

    pub(crate) fn battle(
        attacker: (Player, &Card),
        defender: (Player, &Card),
        result: &BattleResult,
    ) -> Self {
        let attacker = (attacker.0, attacker.1.clone());
        let defender = (defender.0, defender.1.clone());
        let result = result.clone();
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
    pub(crate) fn new(turn: Player) -> Self {
        let mut entries = Vec::new();
        entries.push(Entry::next_turn(turn));
        GameLog { entries }
    }

    pub(crate) fn append(&mut self, entry: Entry) {
        self.entries.push(entry)
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}
