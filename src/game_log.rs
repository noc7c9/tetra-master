use crate::{Card, Player};

pub(crate) enum EntryData {
    PlaceCard { card: Card, cell: usize },
    FlipCard { card: Card, cell: usize, to: Player },
}

pub(crate) struct Entry {
    pub(crate) turn: u8,
    pub(crate) player: Player,
    pub(crate) data: EntryData,
}

pub(crate) struct GameLog {
    turn: u8,
    player: Player,
    entries: Vec<Entry>,
}

impl GameLog {
    pub(crate) fn new(player: Player) -> Self {
        GameLog {
            turn: 1,
            player,
            entries: Vec::new(),
        }
    }

    pub(crate) fn next_turn(&mut self, player: Player) {
        self.turn += 1;
        self.player = player;
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
            turn: self.turn,
            player: self.player,
            data,
        })
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}
