use crate::{
    driver::{BattleWinner, Battler, Command, Digit, Event, Response},
    Arrows, BattleSystem, Card, HandCandidate, HandCandidates, Player, Rng, Seed,
};

pub(super) const CARD: Card = Card::physical(0, 0, 0, 0);
pub(super) const HAND_CANDIDATES: HandCandidates = {
    const HAND: HandCandidate = [CARD, CARD, CARD, CARD, CARD];
    [HAND, HAND, HAND]
};

impl Command {
    pub(super) fn setup() -> Self {
        Command::Setup {
            rng: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        }
    }

    pub(super) fn pick_hand(hand: u8) -> Self {
        Command::PickHand { hand }
    }

    pub(super) fn place_card(card: u8, cell: u8) -> Self {
        Command::PlaceCard { card, cell }
    }

    pub(super) fn pick_battle(cell: u8) -> Self {
        Command::PickBattle { cell }
    }

    pub(super) fn seed(mut self, seed: Seed) -> Self {
        if let Command::Setup { ref mut rng, .. } = self {
            *rng = Some(Rng::Seeded { seed });
            self
        } else {
            panic!("Cannot set field rng on {self:?}")
        }
    }

    pub(super) fn rolls(mut self, rolls: &[u8]) -> Self {
        if let Command::Setup { ref mut rng, .. } = self {
            let rolls = rolls.into();
            *rng = Some(Rng::External { rolls });
            self
        } else {
            panic!("Cannot set field rng on {self:?}")
        }
    }

    pub(super) fn battle_system(mut self, value: BattleSystem) -> Self {
        if let Command::Setup {
            ref mut battle_system,
            ..
        } = self
        {
            *battle_system = Some(value);
            self
        } else {
            panic!("Cannot set field battle_system on {self:?}")
        }
    }

    pub(super) fn blocked_cells(mut self, value: &[u8]) -> Self {
        if let Command::Setup {
            ref mut blocked_cells,
            ..
        } = self
        {
            *blocked_cells = Some(value.into());
            self
        } else {
            panic!("Cannot set field blocked_cells on {self:?}")
        }
    }

    pub(super) fn hand_candidates(mut self, value: &HandCandidates) -> Self {
        if let Command::Setup {
            ref mut hand_candidates,
            ..
        } = self
        {
            *hand_candidates = Some(*value);
            self
        } else {
            panic!("Cannot set field hand_candidates on {self:?}")
        }
    }
}

pub(super) struct SetupOk {
    pub(super) seed: Option<Seed>,
    // pub(super) battle_system: BattleSystem,
    pub(super) blocked_cells: Vec<u8>,
    pub(super) hand_candidates: HandCandidates,
}

impl Response {
    pub(super) fn setup_ok(self) -> SetupOk {
        if let Response::SetupOk {
            seed,
            // battle_system,
            blocked_cells,
            hand_candidates,
            ..
        } = self
        {
            SetupOk {
                seed,
                // battle_system,
                blocked_cells,
                hand_candidates,
            }
        } else {
            panic!("Expected Response::SetupOk, found {self:?}")
        }
    }

    // pub(super) fn pick_hand_ok(self) {
    //     if let Response::PickHandOk = self {
    //     } else {
    //         panic!("Expected Response::PickHandOk, found {self:?}")
    //     }
    // }

    pub(super) fn pick_hand_err(self) -> String {
        if let Response::PickHandErr { reason } = self {
            reason
        } else {
            panic!("Expected Response::PickHandErr, found {self:?}")
        }
    }

    pub(super) fn place_card_ok(self) -> Vec<Event> {
        if let Response::PlaceCardOk { events } = self {
            events
        } else {
            panic!("Expected Response::PlaceCardOk, found {self:?}")
        }
    }

    pub(super) fn place_card_pick_battle(self) -> Vec<u8> {
        if let Response::PlaceCardPickBattle { choices } = self {
            choices
        } else {
            panic!("Expected Response::PlaceCardPickBattle, found {self:?}")
        }
    }
}

impl Event {
    pub(super) fn flip(cell: u8) -> Self {
        Event::Flip { cell }
    }

    pub(super) fn combo_flip(cell: u8) -> Self {
        Event::ComboFlip { cell }
    }

    pub(super) fn battle(attacker: Battler, defender: Battler, winner: BattleWinner) -> Self {
        Event::Battle {
            attacker,
            defender,
            winner,
        }
    }

    pub(super) fn game_over(winner: Option<Player>) -> Self {
        Event::GameOver { winner }
    }
}

impl Battler {
    pub(super) fn new(cell: u8, digit: Digit, value: u8, roll: u8) -> Self {
        Self {
            cell,
            digit,
            value,
            roll,
        }
    }
}

impl Card {
    pub(super) fn arrows(mut self, arrows: Arrows) -> Self {
        self.arrows = arrows;
        self
    }
}
