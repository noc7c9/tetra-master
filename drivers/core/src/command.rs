use crate::{BattleSystem, Card, CardType, HandCandidates};
use std::fmt::Result as FResult;

// TODO: replace this with a bespoke Error enum
pub type Error = std::fmt::Error;

pub trait Command {
    fn serialize(self, output: &mut String) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Setup {
    pub battle_system: BattleSystem,
    pub blocked_cells: Vec<u8>,
    pub hand_candidates: HandCandidates,
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
                    BattleSystem::Test => o.atom("test"),
                }
            })?;

            o.list(|o| {
                o.atom("blocked-cells")?;
                write_blocked_cells(o, &self.blocked_cells)
            })?;

            o.list(|o| {
                o.atom("hand-candidates")?;
                write_hand_candidates(o, &self.hand_candidates)
            })?;

            Ok(())
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug)]
pub struct PushRngNumbers {
    pub numbers: Vec<u8>,
}

impl Command for PushRngNumbers {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("push-rng-numbers")?;

            if !self.numbers.is_empty() {
                o.list(|o| {
                    o.atom("numbers")?;
                    o.array(self.numbers, |o, number| o.atom(DisplayHex(number)))
                })?;
            }

            Ok(())
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug)]
pub struct PickHand {
    pub hand: u8,
}

impl Command for PickHand {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("pick-hand")?;
            o.list(|o| o.atoms(("hand", self.hand)))
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug)]
pub struct PlaceCard {
    pub card: u8,
    pub cell: u8,
}

impl Command for PlaceCard {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("place-card")?;
            o.list(|o| o.atoms(("card", self.card)))?;
            o.list(|o| o.atoms(("cell", DisplayHex(self.cell))))
        })?;

        out.push('\n');

        Ok(())
    }
}

#[derive(Debug)]
pub struct PickBattle {
    pub cell: u8,
}

impl Command for PickBattle {
    fn serialize(self, out: &mut String) -> Result<(), Error> {
        let mut o = Sexpr::new(out);

        o.list(|o| {
            o.atom("pick-battle")?;
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

fn write_blocked_cells(o: &mut Sexpr, blocked_cells: &[u8]) -> FResult {
    o.array(blocked_cells, |o, cell| o.atom(DisplayHex(cell)))
}

fn write_hand_candidates(o: &mut Sexpr, hand_candidates: &HandCandidates) -> FResult {
    o.array(hand_candidates, |o, hand| {
        o.array(hand, |o, card| write_card(o, *card))
    })
}

struct DisplayHex<T>(T);

impl<T: std::fmt::UpperHex> std::fmt::Display for DisplayHex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> FResult {
        write!(f, "{:X}", self.0)
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
    use crate::{Arrows, BattleSystem, Card, HandCandidates};

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

    #[test_case(vec![] => using assert_eq("()"))]
    #[test_case(vec![1] => using assert_eq("(1)"))]
    #[test_case(vec![0xa, 0xf, 3] => using assert_eq("(A F 3)"))]
    #[test_case(vec![1, 2, 3, 4, 5, 6] => using assert_eq("(1 2 3 4 5 6)"))]
    fn write_blocked_cells(input: Vec<u8>) -> String {
        let mut o = String::new();
        super::write_blocked_cells(&mut Sexpr::new(&mut o), &input).unwrap();
        o
    }

    #[test_case([
        [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
        [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
        [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
    ] => using assert_eq(concat!("((1P23_4 5M67_8 9XAB_C DAEF_0 5M67_8)",
                                 " (5M67_8 1P23_4 DAEF_0 5M67_8 9XAB_C)",
                                 " (DAEF_0 5M67_8 9XAB_C 5M67_8 1P23_4))")))]
    fn write_hand_candidates(input: HandCandidates) -> String {
        let mut o = String::new();
        super::write_hand_candidates(&mut Sexpr::new(&mut o), &input).unwrap();
        o
    }

    #[test_case(Setup {
        battle_system: BattleSystem::Dice { sides: 8 },
        blocked_cells: vec![2, 8, 0xA],
        hand_candidates: [
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
            [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
        ],
    } => using assert_eq(concat!("(setup (battle-system dice 8)",
                                       " (blocked-cells (2 8 A))",
                                       " (hand-candidates ",
                                          "((1P23_4 5M67_8 9XAB_C DAEF_0 5M67_8)",
                                          " (5M67_8 1P23_4 DAEF_0 5M67_8 9XAB_C)",
                                          " (DAEF_0 5M67_8 9XAB_C 5M67_8 1P23_4))))\n")))]
    fn setup(input: Setup) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PushRngNumbers {
        numbers: vec![24, 3, 5, 2, 134, 3, 5, 2, 94, 4],
    } => using assert_eq("(push-rng-numbers (numbers (18 3 5 2 86 3 5 2 5E 4)))\n"))]
    #[test_case(PushRngNumbers {
        numbers: vec![],
    } => using assert_eq("(push-rng-numbers)\n"))]
    fn push_rng_numbers(input: PushRngNumbers) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PickHand { hand: 0 } => using assert_eq("(pick-hand (hand 0))\n"))]
    #[test_case(PickHand { hand: 1 } => using assert_eq("(pick-hand (hand 1))\n"))]
    #[test_case(PickHand { hand: 2 } => using assert_eq("(pick-hand (hand 2))\n"))]
    fn pick_hand(input: PickHand) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PlaceCard { card: 0, cell: 0 }
        => using assert_eq("(place-card (card 0) (cell 0))\n"))]
    #[test_case(PlaceCard { card: 3, cell: 0xA }
        => using assert_eq("(place-card (card 3) (cell A))\n"))]
    fn place_card(input: PlaceCard) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }

    #[test_case(PickBattle { cell: 0 } => using assert_eq("(pick-battle (cell 0))\n"))]
    #[test_case(PickBattle { cell: 0xA } => using assert_eq("(pick-battle (cell A))\n"))]
    fn pick_battle(input: PickBattle) -> String {
        let mut o = String::new();
        input.serialize(&mut o).unwrap();
        o
    }
}
