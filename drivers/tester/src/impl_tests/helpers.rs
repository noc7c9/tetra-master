use tetra_master_core::{
    command, Arrows, BattleSystem, BattleWinner, Battler, BoardCells, Card, Driver, DriverBuilder,
    Error, ErrorResponse, Event, Hand, Player,
};

pub(super) const CARD: Card = Card::physical(0, 0, 0, Arrows(0));
pub(super) const HAND_BLUE: Hand = [CARD, CARD, CARD, CARD, CARD];
pub(super) const HAND_RED: Hand = [CARD, CARD, CARD, CARD, CARD];

pub(super) trait DriverBuilderExt {
    fn build_with_rng(self, numbers: &[u8]) -> Driver;
}

impl DriverBuilderExt for DriverBuilder {
    fn build_with_rng(self, numbers: &[u8]) -> Driver {
        let mut driver = self.no_auto_feed_rng().build();
        driver
            .send(command::PushRngNumbers {
                numbers: numbers.to_vec(),
            })
            .unwrap();
        driver
    }
}

pub(super) struct Command;

impl Command {
    pub(super) fn setup() -> command::Setup {
        command::Setup {
            battle_system: BattleSystem::Test,
            blocked_cells: BoardCells::NONE,
            hand_blue: HAND_BLUE,
            hand_red: HAND_RED,
        }
    }

    pub(super) fn place_card(card: u8, cell: u8) -> command::PlaceCard {
        command::PlaceCard { card, cell }
    }

    pub(super) fn pick_battle(cell: u8) -> command::PickBattle {
        command::PickBattle { cell }
    }
}

pub(super) trait SetupExt {
    fn battle_system(self, value: BattleSystem) -> Self;
    fn blocked_cells(self, value: &[u8]) -> Self;
    fn hand_blue(self, value: &Hand) -> Self;
    fn hand_red(self, value: &Hand) -> Self;
}

impl SetupExt for command::Setup {
    fn battle_system(mut self, value: BattleSystem) -> Self {
        self.battle_system = value;
        self
    }

    fn blocked_cells(mut self, value: &[u8]) -> Self {
        self.blocked_cells = value.into();
        self
    }

    fn hand_blue(mut self, value: &Hand) -> Self {
        self.hand_blue = *value;
        self
    }

    fn hand_red(mut self, value: &Hand) -> Self {
        self.hand_red = *value;
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
