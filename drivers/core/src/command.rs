use crate::{display::DisplayHex, BattleSystem, BoardCells, Card, CardType, Hand, Player};
use std::fmt::Result as FResult;

// TODO: replace this with a bespoke Error enum
pub type Error = std::fmt::Error;

pub trait Command {
    fn serialize(self, output: &mut String) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Setup {
    pub battle_system: BattleSystem,
    pub blocked_cells: BoardCells,
    pub hand_blue: Hand,
    pub hand_red: Hand,
    pub starting_player: Player,
}

impl Command for Setup {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("setup")?;

            o.list(|o| {
                o.atom("battle-system")?;
                match self.battle_system {
                    BattleSystem::Original => o.atom("original"),
                    BattleSystem::Dice { sides } => o.atoms(("dice", DisplayHex(sides))),
                    BattleSystem::Deterministic => o.atom("deterministic"),
                    BattleSystem::Test => o.atom("test"),
                }
            })?;

            o.list(|o| {
                o.atom("blocked-cells")?;
                write_blocked_cells(o, self.blocked_cells)
            })?;

            o.list(|o| {
                o.atom("hand-blue")?;
                write_hand(o, &self.hand_blue)
            })?;

            o.list(|o| {
                o.atom("hand-red")?;
                write_hand(o, &self.hand_red)
            })?;

            o.list(|o| {
                o.atom("starting-player")?;
                write_player(o, self.starting_player)
            })?;

            Ok(())
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceCard {
    pub player: Player,
    pub card: u8,
    pub cell: u8,
}

impl Command for PlaceCard {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("place-card")?;

            o.list(|o| {
                o.atom("player")?;
                write_player(o, self.player)
            })?;
            o.list(|o| o.atoms(("card", self.card)))?;
            o.list(|o| o.atoms(("cell", DisplayHex(self.cell))))
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug)]
pub struct ResolveBattle {
    pub attack_roll: Vec<u8>,
    pub defend_roll: Vec<u8>,
}

impl Command for ResolveBattle {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("resolve-battle")?;
            o.list(|o| {
                o.atom("attack")?;
                o.array(self.attack_roll, |o, num| o.atom(DisplayHex(num)))
            })?;
            o.list(|o| {
                o.atom("defend")?;
                o.array(self.defend_roll, |o, num| o.atom(DisplayHex(num)))
            })
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickBattle {
    pub player: Player,
    pub cell: u8,
}

impl Command for PickBattle {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("pick-battle")?;
            o.list(|o| {
                o.atom("player")?;
                write_player(o, self.player)
            })?;
            o.list(|o| o.atoms(("cell", DisplayHex(self.cell))))
        })?;

        out.push('\n');

        Ok(())
    }
}

fn write_card(o: &mut Sexpr, card: Card) -> FResult {
    let att = card.attack;
    let phy = card.physical_defense;
    let mag = card.magical_defense;
    let typ = match card.card_type {
        CardType::Physical => 'P',
        CardType::Magical => 'M',
        CardType::Exploit => 'X',
        CardType::Assault => 'A',
    };
    let arr = card.arrows.0;
    o.atom_fmt(format_args!("{att:X}{typ}{phy:X}{mag:X}_{arr:X}"))
}

fn write_blocked_cells(o: &mut Sexpr, blocked_cells: BoardCells) -> FResult {
    o.array(blocked_cells, |o, cell| o.atom(DisplayHex(cell)))
}

fn write_hand(o: &mut Sexpr, hand: &Hand) -> FResult {
    o.array(hand, |o, card| write_card(o, *card))
}

fn write_player(o: &mut Sexpr, player: Player) -> FResult {
    match player {
        Player::Blue => o.atom("blue"),
        Player::Red => o.atom("red"),
    }
}

use sexpr::Sexpr;
mod sexpr {
    use std::fmt::{Display, Result, Write};

    pub struct Sexpr<'i> {
        inner: &'i mut String,
        is_list_empty: Vec<bool>,
    }

    impl<'i> Sexpr<'i> {
        pub fn new(inner: &'i mut String) -> Self {
            Self {
                inner,
                is_list_empty: vec![true],
            }
        }

        pub fn list(&mut self, contents: impl FnOnce(&mut Self) -> Result) -> Result {
            self.list_start()?;
            contents(self)?;
            self.list_end()
        }

        pub fn atom<T: Display>(&mut self, atom: T) -> Result {
            self.space()?;
            write!(self.inner, "{}", atom)
        }

        pub fn atoms<T: AtomTuple>(&mut self, tuple: T) -> Result {
            tuple.atom_each(self)
        }

        pub fn atom_fmt(&mut self, args: std::fmt::Arguments) -> Result {
            self.space()?;
            self.inner.write_fmt(args)
        }

        pub fn array<T>(
            &mut self,
            items: impl IntoIterator<Item = T>,
            f: impl Fn(&mut Self, T) -> Result,
        ) -> Result {
            self.list_start()?;
            for item in items {
                f(self, item)?;
            }
            self.list_end()
        }

        fn list_start(&mut self) -> Result {
            self.space()?;
            self.inner.write_char('(')?;
            self.is_list_empty.push(true);
            Ok(())
        }

        fn list_end(&mut self) -> Result {
            self.is_list_empty.pop();
            self.inner.write_char(')')
        }

        fn space(&mut self) -> Result {
            let is_list_empty = self.is_list_empty.last_mut().unwrap();
            if *is_list_empty {
                *is_list_empty = false;
                Ok(())
            } else {
                self.inner.write_char(' ')
            }
        }
    }

    pub trait AtomTuple {
        fn atom_each(&self, o: &mut Sexpr) -> Result;
    }

    impl<A: Display, B: Display> AtomTuple for (A, B) {
        fn atom_each(&self, o: &mut Sexpr) -> Result {
            o.atom(&self.0)?;
            o.atom(&self.1)
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{Arrows, BattleSystem, Card, Hand};

    fn assert_eq<T, U>(expected: T) -> impl Fn(U)
    where
        T: std::fmt::Debug,
        U: PartialEq<T> + std::fmt::Debug,
    {
        move |actual| pretty_assertions::assert_eq!(actual, expected)
    }

    const C1P23_4: Card = Card::physical(1, 2, 3, Arrows(4));
    const C5M67_8: Card = Card::magical(5, 6, 7, Arrows(8));
    const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, Arrows(0xC));
    const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, Arrows(0));
    const C0P00_0F: Card = Card::physical(0, 0, 0, Arrows(0xF));
    const C0P00_A0: Card = Card::physical(0, 0, 0, Arrows(0xA0));
    const C0P00_FA: Card = Card::physical(0, 0, 0, Arrows(0xFA));

    #[test_case(C1P23_4 => using assert_eq("1P23_4"))]
    #[test_case(C5M67_8 => using assert_eq("5M67_8"))]
    #[test_case(C9XAB_C => using assert_eq("9XAB_C"))]
    #[test_case(CDAEF_0 => using assert_eq("DAEF_0"))]
    #[test_case(C0P00_0F => using assert_eq("0P00_F"))]
    #[test_case(C0P00_A0 => using assert_eq("0P00_A0"))]
    #[test_case(C0P00_FA => using assert_eq("0P00_FA"))]
    fn write_card(input: Card) -> String {
        let mut o = String::new();
        super::write_card(&mut Sexpr::new(&mut o), input).unwrap();
        o
    }

    #[test_case([] => using assert_eq("()"))]
    #[test_case([1] => using assert_eq("(1)"))]
    #[test_case([0xa, 0xf, 3] => using assert_eq("(3 A F)"))]
    #[test_case([1, 2, 3, 4, 5, 6] => using assert_eq("(1 2 3 4 5 6)"))]
    fn write_blocked_cells(input: impl Into<BoardCells>) -> String {
        let mut o = String::new();
        super::write_blocked_cells(&mut Sexpr::new(&mut o), input.into()).unwrap();
        o
    }

    #[test_case([C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8]
        => using assert_eq("(1P23_4 5M67_8 9XAB_C DAEF_0 5M67_8)"))]
    #[test_case([C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C]
        => using assert_eq("(5M67_8 1P23_4 DAEF_0 5M67_8 9XAB_C)"))]
    #[test_case([CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4]
        => using assert_eq("(DAEF_0 5M67_8 9XAB_C 5M67_8 1P23_4)"))]
    fn write_hand(input: Hand) -> String {
        let mut o = String::new();
        super::write_hand(&mut Sexpr::new(&mut o), &input).unwrap();
        o
    }

    #[test_case(Player::Blue => using assert_eq("blue"))]
    #[test_case(Player::Red => using assert_eq("red"))]
    fn write_player(input: Player) -> String {
        let mut o = String::new();
        super::write_player(&mut Sexpr::new(&mut o), input).unwrap();
        o
    }

    #[test_case(Setup {
        battle_system: BattleSystem::Dice { sides: 8 },
        blocked_cells: [2, 8, 0xA].into(),
        hand_blue: [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
        hand_red: [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
        starting_player: Player::Blue
    } => using assert_eq(concat!("(setup (battle-system dice 8)",
                                       " (blocked-cells (2 8 A))",
                                       " (hand-blue (1P23_4 5M67_8 9XAB_C DAEF_0 5M67_8))",
                                       " (hand-red (5M67_8 1P23_4 DAEF_0 5M67_8 9XAB_C))",
                                       " (starting-player blue))\n")))]
    fn setup(input: Setup) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PlaceCard { player: Player::Blue, card: 0, cell: 0 }
        => using assert_eq("(place-card (player blue) (card 0) (cell 0))\n"))]
    #[test_case(PlaceCard { player: Player::Red, card: 3, cell: 0xA }
        => using assert_eq("(place-card (player red) (card 3) (cell A))\n"))]
    fn place_card(input: PlaceCard) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PickBattle { player: Player::Blue, cell: 0 }
        => using assert_eq("(pick-battle (player blue) (cell 0))\n"))]
    #[test_case(PickBattle { player: Player::Red, cell: 0xA }
        => using assert_eq("(pick-battle (player red) (cell A))\n"))]
    fn pick_battle(input: PickBattle) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(ResolveBattle { attack_roll: vec![1, 2, 8, 129], defend_roll: vec![2, 3, 233] }
        => using assert_eq("(resolve-battle (attack (1 2 8 81)) (defend (2 3 E9)))\n"))]
    fn resolve_battle(input: ResolveBattle) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }
}
