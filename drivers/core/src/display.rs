/// Display impls for all the responses and commands
use crate::{command, response, Arrows, Battler, Card, CardType, Event, Hand, Player};
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

fn write_arrows(f: &mut std::fmt::Formatter<'_>, arrows: Arrows) -> std::fmt::Result {
    let mut iter = [
        ("U", Arrows::UP),
        ("UR", Arrows::UP_RIGHT),
        ("R", Arrows::RIGHT),
        ("DR", Arrows::DOWN_RIGHT),
        ("D", Arrows::DOWN),
        ("DL", Arrows::DOWN_LEFT),
        ("L", Arrows::LEFT),
        ("UL", Arrows::UP_LEFT),
    ]
    .into_iter()
    .filter(|(_, n)| arrows.has_any(*n));
    if let Some((s, _)) = iter.next() {
        write!(f, "{s}")?;
    }
    for (s, _) in iter {
        write!(f, "|{s}")?;
    }
    Ok(())
}

impl Display for Arrows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arrows(")?;
        write_arrows(f, *self)?;
        write!(f, ")")
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
        write!(f, "{att:X}{ctype}{phy:X}{mag:X}_")?;
        write_arrows(f, self.arrows)
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::Blue => write!(f, "Blue"),
            Player::Red => write!(f, "Red"),
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
            "Setup battle-system:{:?} blocked-cells:{} hand-blue:{} hand-red:{} starting-player:{}",
            self.battle_system,
            DisplayList::new(self.blocked_cells.into_iter().map(DisplayHex)),
            DisplayHand(&self.hand_blue),
            DisplayHand(&self.hand_red),
            &self.starting_player,
        )
    }
}

impl Display for command::PlaceCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PlaceCard player:{} card:{:X} cell:{:X}",
            self.player, self.card, self.cell
        )
    }
}

impl Display for command::PickBattle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PickBattle player:{} cell:{:X}", self.player, self.cell)
    }
}

impl Display for command::ResolveBattle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResolveBattle attack_roll:{:X?} defend_roll:{:X?}",
            self.attack_roll, self.defend_roll
        )
    }
}

impl Display for response::SetupOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetupOk")
    }
}

impl std::fmt::Display for response::PlayOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlayOk")?;
        if let Some(resolve_battle) = &self.resolve_battle {
            write!(f, " resolve-battle")?;
            let atk = resolve_battle.attack_roll;
            write!(
                f,
                " Attack({}x({}..{}))",
                atk.numbers, atk.range.0, atk.range.1
            )?;
            let def = resolve_battle.defend_roll;
            write!(
                f,
                " Defend({}x({}..{}))",
                def.numbers, def.range.0, def.range.1
            )?;
        }
        if !self.pick_battle.is_empty() {
            write!(
                f,
                " pick-battle:{}",
                DisplayList::new(self.pick_battle.into_iter().map(DisplayHex))
            )?;
        }
        write!(f, " events:{}", DisplayList::new(self.events.iter()))
    }
}
