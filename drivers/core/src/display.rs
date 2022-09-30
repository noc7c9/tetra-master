/// Display impls for all the responses and commands
use crate::{command, response, Battler, Card, CardType, Event, Hand, Player};
use std::fmt::Display;

pub(crate) struct DisplayHex<T>(pub T);

impl<T> Display for DisplayHex<T>
where
    T: std::fmt::UpperHex,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

struct DisplayList<T>(std::cell::RefCell<T>);

impl<T> DisplayList<T> {
    fn new(inner: T) -> Self {
        Self(std::cell::RefCell::new(inner))
    }
}

impl<T> Display for DisplayList<T>
where
    T: Iterator,
    T::Item: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::ops::DerefMut;

        let mut iter = self.0.borrow_mut();

        write!(f, "[")?;
        if let Some(item) = iter.next() {
            write!(f, "{item}")?;
            for item in iter.deref_mut() {
                write!(f, " {item}")?;
            }
        }
        write!(f, "]")
    }
}

struct DisplayHand<'a>(&'a Hand);

impl<'a> Display for DisplayHand<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", DisplayList::new(self.0.iter()))
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let att = self.attack;
        let phy = self.physical_defense;
        let mag = self.magical_defense;
        let ctype = match self.card_type {
            CardType::Physical => 'P',
            CardType::Magical => 'M',
            CardType::Exploit => 'X',
            CardType::Assault => 'A',
        };
        write!(f, "{att:X}{ctype}{phy:X}{mag:X}_{:X}", self.arrows.0)
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::P1 => write!(f, "P1"),
            Player::P2 => write!(f, "P2"),
        }
    }
}

impl Display for Battler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cell:{} {:?}({}) roll:{}",
            self.cell, self.digit, self.value, self.roll
        )
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::NextTurn { to } => write!(f, "NextTurn({})", to),
            Event::Flip { cell } => write!(f, "Flip({cell:X})"),
            Event::ComboFlip { cell } => write!(f, "ComboFlip({cell:X})"),
            Event::Battle {
                attacker,
                defender,
                winner,
            } => write!(
                f,
                "Battle(Attacker({attacker}) Defender({defender}) Winner({winner:?}))"
            ),
            Event::GameOver { winner } => {
                if let Some(player) = winner {
                    write!(f, "GameOver(winner:{player})")
                } else {
                    write!(f, "GameOver(winner:None)")
                }
            }
        }
    }
}

impl Display for command::Setup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Setup battle-system:{:?} blocked-cells:{} hand-candidates:{}",
            self.battle_system,
            DisplayList::new(self.blocked_cells.iter().map(DisplayHex)),
            DisplayList::new(self.hand_candidates.iter().map(DisplayHand)),
        )
    }
}

impl Display for command::PushRngNumbers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PushRngNumbers {}",
            DisplayList::new(self.numbers.iter().map(DisplayHex)),
        )
    }
}

impl Display for command::PickHand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PickHand hand:{:X}", self.hand)
    }
}

impl Display for command::PlaceCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlaceCard card:{:X} cell:{:X}", self.card, self.cell)
    }
}

impl Display for command::PickBattle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PickBattle cell:{:X}", self.cell)
    }
}

impl Display for response::SetupOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetupOk")
    }
}

impl Display for response::PushRngNumbersOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PushRngNumbersOk numbers-left:{:X}", self.numbers_left)
    }
}

impl Display for response::PickHandOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PickHandOk")
    }
}

impl std::fmt::Display for response::PlayOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlayOk")?;
        if !self.pick_battle.is_empty() {
            write!(
                f,
                " pick-battle:{}",
                DisplayList::new(self.pick_battle.iter().map(DisplayHex))
            )?;
        }
        write!(f, " events:{}", DisplayList::new(self.events.iter()))
    }
}