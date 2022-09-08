use tetra_master_core::{
    Arrows, BattleSystem, BattleWinner, Battler, Card, Command, Driver, ErrorResponse, Event, Hand,
    HandCandidates, Player, Response, Rng, Seed,
};

pub(super) const CARD: Card = Card::physical(0, 0, 0, Arrows(0));
pub(super) const HAND_CANDIDATES: HandCandidates = {
    const HAND: Hand = [CARD, CARD, CARD, CARD, CARD];
    [HAND, HAND, HAND]
};

pub(super) struct Ctx {
    pub(super) implementation: String,
}

impl Ctx {
    pub(super) fn new_driver(&self) -> Driver {
        Driver::new(&self.implementation)
    }
}

pub(super) trait CommandExt {
    fn setup() -> Self;
    fn pick_hand(hand: u8) -> Self;
    fn place_card(card: u8, cell: u8) -> Self;
    fn pick_battle(cell: u8) -> Self;
    fn seed(self, seed: Seed) -> Self;
    fn rolls(self, rolls: &[u8]) -> Self;
    fn battle_system(self, value: BattleSystem) -> Self;
    fn blocked_cells(self, value: &[u8]) -> Self;
    fn hand_candidates(self, value: &HandCandidates) -> Self;
}

impl CommandExt for Command {
    fn setup() -> Self {
        Command::Setup {
            rng: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        }
    }

    fn pick_hand(hand: u8) -> Self {
        Command::PickHand { hand }
    }

    fn place_card(card: u8, cell: u8) -> Self {
        Command::PlaceCard { card, cell }
    }

    fn pick_battle(cell: u8) -> Self {
        Command::PickBattle { cell }
    }

    fn seed(mut self, seed: Seed) -> Self {
        if let Command::Setup { ref mut rng, .. } = self {
            *rng = Some(Rng::Seeded { seed });
            self
        } else {
            panic!("Cannot set field rng on {self:?}")
        }
    }

    fn rolls(mut self, rolls: &[u8]) -> Self {
        if let Command::Setup { ref mut rng, .. } = self {
            let rolls = rolls.into();
            *rng = Some(Rng::External { rolls });
            self
        } else {
            panic!("Cannot set field rng on {self:?}")
        }
    }

    fn battle_system(mut self, value: BattleSystem) -> Self {
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

    fn blocked_cells(mut self, value: &[u8]) -> Self {
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

    fn hand_candidates(mut self, value: &HandCandidates) -> Self {
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

pub(super) trait ResponseExt {
    fn error(self) -> ErrorResponse;
    fn setup_ok(self) -> SetupOk;
    fn pick_hand_ok(self);
    fn place_card_ok(self) -> (Vec<u8>, Vec<Event>);
}

impl ResponseExt for Response {
    fn error(self) -> ErrorResponse {
        if let Response::Error(inner) = self {
            inner
        } else {
            panic!("Expected Response::Error, found {self:?}")
        }
    }

    fn setup_ok(self) -> SetupOk {
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

    fn pick_hand_ok(self) {
        if let Response::PickHandOk = self {
        } else {
            panic!("Expected Response::PickHandOk, found {self:?}")
        }
    }

    fn place_card_ok(self) -> (Vec<u8>, Vec<Event>) {
        if let Response::PlaceCardOk {
            pick_battle,
            events,
        } = self
        {
            (pick_battle, events)
        } else {
            panic!("Expected Response::PlaceCardOk, found {self:?}")
        }
    }
}

pub(super) trait EventExt {
    fn turn_p1() -> Self;
    fn turn_p2() -> Self;
    fn flip(cell: u8) -> Self;
    fn combo_flip(cell: u8) -> Self;
    fn battle(attacker: Battler, defender: Battler, winner: BattleWinner) -> Self;
    fn game_over(winner: Option<Player>) -> Self;
    fn as_battle(&self) -> (Battler, Battler, BattleWinner);
}

impl EventExt for Event {
    fn turn_p1() -> Self {
        Event::NextTurn { to: Player::P1 }
    }

    fn turn_p2() -> Self {
        Event::NextTurn { to: Player::P2 }
    }

    fn flip(cell: u8) -> Self {
        Event::Flip { cell }
    }

    fn combo_flip(cell: u8) -> Self {
        Event::ComboFlip { cell }
    }

    fn battle(attacker: Battler, defender: Battler, winner: BattleWinner) -> Self {
        Event::Battle {
            attacker,
            defender,
            winner,
        }
    }

    fn game_over(winner: Option<Player>) -> Self {
        Event::GameOver { winner }
    }

    fn as_battle(&self) -> (Battler, Battler, BattleWinner) {
        if let Event::Battle {
            attacker,
            defender,
            winner,
        } = *self
        {
            (attacker, defender, winner)
        } else {
            panic!("Expected Event::Battle, found {self:?}")
        }
    }
}

pub(super) trait CardExt {
    fn arrows(self, arrows: Arrows) -> Self;
}

impl CardExt for Card {
    fn arrows(mut self, arrows: Arrows) -> Self {
        self.arrows = arrows;
        self
    }
}
