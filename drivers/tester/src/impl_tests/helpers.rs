use tetra_master_core::{
    command, Arrows, BattleSystem, BattleWinner, Battler, BoardCells, Card, Error, ErrorResponse,
    Event, Hand, Player,
};

pub(super) const CARD: Card = Card::physical(0, 0, 0, Arrows(0));
pub(super) const HAND_BLUE: Hand = [CARD, CARD, CARD, CARD, CARD];
pub(super) const HAND_RED: Hand = [CARD, CARD, CARD, CARD, CARD];

pub(super) struct Command;

impl Command {
    pub(super) fn setup() -> command::Setup {
        command::Setup {
            battle_system: BattleSystem::Test,
            blocked_cells: BoardCells::NONE,
            hand_blue: HAND_BLUE,
            hand_red: HAND_RED,
            starting_player: Player::Blue,
        }
    }

    pub(super) fn place_card_blue(card: u8, cell: u8) -> command::PlaceCard {
        command::PlaceCard {
            player: Player::Blue,
            card,
            cell,
        }
    }

    pub(super) fn pick_battle_blue(cell: u8) -> command::PickBattle {
        command::PickBattle {
            player: Player::Blue,
            cell,
        }
    }

    pub(super) fn place_card_red(card: u8, cell: u8) -> command::PlaceCard {
        command::PlaceCard {
            player: Player::Red,
            card,
            cell,
        }
    }

    pub(super) fn pick_battle_red(cell: u8) -> command::PickBattle {
        command::PickBattle {
            player: Player::Red,
            cell,
        }
    }

    pub(super) fn resolve_battle(attack_roll: &[u8], defend_roll: &[u8]) -> command::ResolveBattle {
        command::ResolveBattle {
            attack_roll: attack_roll.to_vec(),
            defend_roll: defend_roll.to_vec(),
        }
    }
}

pub(super) trait SetupExt {
    fn battle_system(self, value: BattleSystem) -> Self;
    fn blocked_cells(self, value: &[u8]) -> Self;
    fn hand_blue(self, value: &Hand) -> Self;
    fn hand_red(self, value: &Hand) -> Self;
    fn starting_player(self, value: Player) -> Self;
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

    fn starting_player(mut self, value: Player) -> Self {
        self.starting_player = value;
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
    fn turn_blue() -> Self;
    fn turn_red() -> Self;
    fn flip(cell: u8) -> Self;
    fn combo_flip(cell: u8) -> Self;
    fn battle(attacker: Battler, defender: Battler, winner: BattleWinner) -> Self;
    fn game_over(winner: Option<Player>) -> Self;
    fn as_battle(&self) -> (Battler, Battler, BattleWinner);
}

impl EventExt for Event {
    fn turn_blue() -> Self {
        Event::NextTurn { to: Player::Blue }
    }

    fn turn_red() -> Self {
        Event::NextTurn { to: Player::Red }
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
