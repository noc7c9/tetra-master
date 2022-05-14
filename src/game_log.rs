use crate::{Card, Player};

pub(crate) enum EntryData {
    PlaceCard { card: Card, cell: usize },
    FlipCard { card: Card, cell: usize, to: Player },
}

pub(crate) struct Entry {
    pub(crate) turn_number: u8,
    pub(crate) turn: Player,
    pub(crate) data: EntryData,
}

pub(crate) struct GameLog {
    turn_number: u8,
    turn: Player,
    entries: Vec<Entry>,
}

impl GameLog {
    pub(crate) fn new(turn: Player) -> Self {
        GameLog {
            turn_number: 1,
            turn,
            entries: Vec::new(),
        }
    }

    pub(crate) fn next_turn(&mut self, turn: Player) {
        self.turn_number += 1;
        self.turn = turn;
    }

    pub(crate) fn append_place_card(&mut self, card: &Card, cell: usize) {
        self.append(EntryData::PlaceCard {
            card: card.clone(),
            cell,
        })
    }

    pub(crate) fn append_flip_card(&mut self, card: &Card, cell: usize, to: Player) {
        self.append(EntryData::FlipCard {
            card: card.clone(),
            cell,
            to,
        })
    }

    fn append(&mut self, data: EntryData) {
        self.entries.push(Entry {
            turn_number: self.turn_number,
            turn: self.turn,
            data,
        })
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}
