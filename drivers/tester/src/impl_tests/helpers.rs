use tetra_master_core::{
    command, Arrows, BattleSystem, BattleWinner, Battler, Card, Driver, Error, ErrorResponse,
    Event, Hand, HandCandidates, Player, Rng, Seed,
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

pub(super) struct Command;

impl Command {
    pub(super) fn setup() -> command::Setup {
        command::Setup {
            rng: None,
            battle_system: None,
            blocked_cells: None,
            hand_candidates: None,
        }
    }

    pub(super) fn pick_hand(hand: u8) -> command::PickHand {
        command::PickHand { hand }
    }

    pub(super) fn place_card(card: u8, cell: u8) -> command::PlaceCard {
        command::PlaceCard { card, cell }
    }

    pub(super) fn pick_battle(cell: u8) -> command::PickBattle {
        command::PickBattle { cell }
    }
}

pub(super) trait SetupExt {
    fn seed(self, seed: Seed) -> Self;
    fn rolls(self, rolls: &[u8]) -> Self;
    fn battle_system(self, value: BattleSystem) -> Self;
    fn blocked_cells(self, value: &[u8]) -> Self;
    fn hand_candidates(self, value: &HandCandidates) -> Self;
}

impl SetupExt for command::Setup {
    fn seed(mut self, seed: Seed) -> Self {
        self.rng = Some(Rng::Seeded { seed });
        self
    }

    fn rolls(mut self, rolls: &[u8]) -> Self {
        let rolls = rolls.into();
        self.rng = Some(Rng::External { rolls });
        self
    }

    fn battle_system(mut self, value: BattleSystem) -> Self {
        self.battle_system = Some(value);
        self
    }

    fn blocked_cells(mut self, value: &[u8]) -> Self {
        self.blocked_cells = Some(value.into());
        self
    }

    fn hand_candidates(mut self, value: &HandCandidates) -> Self {
        self.hand_candidates = Some(*value);
        self
    }
}

pub(super) trait ResponseResultExt {
    fn error(self) -> ErrorResponse;
}

impl<T: std::fmt::Debug> ResponseResultExt for Result<T, Error> {
    fn error(self) -> ErrorResponse {
        if let Err(Error::ErrorResponse(inner)) = self {
            inner
        } else {
            panic!("Expected Response::Error, found {self:?}")
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
