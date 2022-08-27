use crate::{BattleSystem, Card, CardType, HandCandidates, Rng};
use std::fmt::{Result, Write};

#[derive(Debug)]
pub(crate) enum Command {
    Quit,
    Setup {
        rng: Option<Rng>,
        battle_system: Option<BattleSystem>,
        blocked_cells: Option<Vec<u8>>,
        hand_candidates: Option<HandCandidates>,
    },
    PickHand {
        index: usize,
    },
    PlaceCard {
        card: usize,
        cell: usize,
    },
    PickBattle {
        cell: usize,
    },
}

impl Command {
    pub(crate) fn serialize(self, out: &mut String) -> anyhow::Result<()> {
        match self {
            Command::Quit => out.write_str("quit")?,
            Command::Setup {
                rng,
                battle_system,
                blocked_cells,
                hand_candidates,
            } => {
                out.write_str("setup")?;
                if let Some(rng) = rng {
                    write!(out, " rng=")?;
                    write_rng(out, &rng)?;
                }
                if let Some(battle_system) = battle_system {
                    write!(out, " battle_system=")?;
                    write_battle_system(out, &battle_system)?;
                }
                if let Some(blocked_cells) = blocked_cells {
                    write!(out, " blocked_cells=")?;
                    write_blocked_cells(out, &blocked_cells)?;
                }
                if let Some(hand_candidates) = hand_candidates {
                    write!(out, " hand_candidates=")?;
                    write_hand_candidates(out, &hand_candidates)?;
                }
                out.write_char('\n')?;
            }
            Command::PickHand { index } => {
                writeln!(out, "pick-hand index={index}")?;
            }
            Command::PlaceCard { card, cell } => {
                writeln!(out, "place-card card={card} cell={cell:X}")?;
            }
            Command::PickBattle { cell } => {
                writeln!(out, "pick-battle cell={cell:X}")?;
            }
        }

        Ok(())
    }
}

fn write_list<T>(
    out: &mut String,
    delimiter: char,
    mut iter: impl Iterator<Item = T>,
    write_item: impl Fn(&mut String, T) -> Result,
) -> Result {
    write!(out, "[")?;
    if let Some(item) = iter.next() {
        write_item(out, item)?;
        for item in iter {
            out.write_char(delimiter)?;
            write_item(out, item)?;
        }
    }
    write!(out, "]")
}

fn write_card(out: &mut String, card: Card) -> Result {
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
    write!(out, "{att:X}{typ}{phy:X}{mag:X}@{arr:02X}")
}

fn write_rng(out: &mut String, rng: &Rng) -> Result {
    match rng {
        Rng::Seeded { seed } => write!(out, "seed({seed})"),
        Rng::External { rolls } => {
            write!(out, "external")?;
            write_list(out, ',', rolls.iter(), |out, v| write!(out, "{v}"))
        }
    }
}

fn write_battle_system(out: &mut String, battle_system: &BattleSystem) -> Result {
    match battle_system {
        BattleSystem::Original => write!(out, "original"),
        BattleSystem::OriginalApprox => write!(out, "original-approx"),
        BattleSystem::Dice { sides } => write!(out, "dice({sides})"),
    }
}

fn write_blocked_cells(out: &mut String, blocked_cells: &[u8]) -> Result {
    write_list(out, ',', blocked_cells.iter(), |out, v| {
        write!(out, "{v:X}")
    })
}

fn write_hand_candidates(out: &mut String, hand_candidates: &HandCandidates) -> Result {
    write_list(out, ';', hand_candidates.iter(), |out, hand| {
        write_list(out, ',', hand.iter(), |out, card| write_card(out, *card))
    })
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::Command::{self, *};
    use crate::{BattleSystem, Card, HandCandidates, Rng};

    fn assert_eq<T, U>(expected: T) -> impl Fn(U)
    where
        T: std::fmt::Debug,
        U: PartialEq<T> + std::fmt::Debug,
    {
        move |actual| pretty_assertions::assert_eq!(actual, expected)
    }

    const C1P23_4: Card = Card::physical(1, 2, 3, 4);
    const C5M67_8: Card = Card::magical(5, 6, 7, 8);
    const C9XAB_C: Card = Card::exploit(9, 0xA, 0xB, 0xC);
    const CDAEF_0: Card = Card::assault(0xD, 0xE, 0xF, 0);
    const C0P00_0F: Card = Card::physical(0, 0, 0, 0xF);
    const C0P00_A0: Card = Card::physical(0, 0, 0, 0xA0);
    const C0P00_FA: Card = Card::physical(0, 0, 0, 0xFA);

    #[test_case(C1P23_4 => using assert_eq("1P23@04"))]
    #[test_case(C5M67_8 => using assert_eq("5M67@08"))]
    #[test_case(C9XAB_C => using assert_eq("9XAB@0C"))]
    #[test_case(CDAEF_0 => using assert_eq("DAEF@00"))]
    #[test_case(C0P00_0F => using assert_eq("0P00@0F"))]
    #[test_case(C0P00_A0 => using assert_eq("0P00@A0"))]
    #[test_case(C0P00_FA => using assert_eq("0P00@FA"))]
    fn write_card(input: Card) -> String {
        let mut out = String::new();
        super::write_card(&mut out, input).unwrap();
        out
    }

    #[test_case(Vec::<u8>::new() => using assert_eq("[]"))]
    #[test_case(vec![1] => using assert_eq("[1]"))]
    #[test_case(vec![0xa, 0xf, 3] => using assert_eq("[A,F,3]"))]
    #[test_case(vec![1, 2, 3, 4, 5, 6] => using assert_eq("[1,2,3,4,5,6]"))]
    fn write_blocked_cells(input: Vec<u8>) -> String {
        let mut out = String::new();
        super::write_blocked_cells(&mut out, &input).unwrap();
        out
    }

    #[test_case([
        [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
        [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
        [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
    ] => using assert_eq("[[1P23@04,5M67@08,9XAB@0C,DAEF@00,5M67@08];[5M67@08,1P23@04,DAEF@00,5M67@08,9XAB@0C];[DAEF@00,5M67@08,9XAB@0C,5M67@08,1P23@04]]"))]
    fn write_hand_candidates(input: HandCandidates) -> String {
        let mut out = String::new();
        super::write_hand_candidates(&mut out, &input).unwrap();
        out
    }

    #[test_case(Setup {
        rng: None,
        battle_system: None,
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup\n"))]
    #[test_case(Setup {
        rng: Some(Rng::Seeded{ seed: 123 }),
        battle_system: None,
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup rng=seed(123)\n"))]
    #[test_case(Setup {
        rng: Some(Rng::External { rolls: vec![2, 3, 5, 2, 1, 3, 5, 2, 4] }),
        battle_system: None,
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup rng=external[2,3,5,2,1,3,5,2,4]\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: Some(BattleSystem::Original),
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup battle_system=original\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: Some(BattleSystem::OriginalApprox),
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup battle_system=original-approx\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: Some(BattleSystem::Dice { sides: 13 }),
        blocked_cells: None,
        hand_candidates: None,
    } => using assert_eq("setup battle_system=dice(13)\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: None,
        blocked_cells: Some(vec![]),
        hand_candidates: None,
    } => using assert_eq("setup blocked_cells=[]\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: None,
        blocked_cells: Some(vec![2]),
        hand_candidates: None,
    } => using assert_eq("setup blocked_cells=[2]\n"))]
    #[test_case(Setup {
        rng: None,
        battle_system: None,
        blocked_cells: None,
        hand_candidates: Some([
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
            [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
        ]),
    } => using assert_eq("setup hand_candidates=[[1P23@04,5M67@08,9XAB@0C,DAEF@00,5M67@08];[5M67@08,1P23@04,DAEF@00,5M67@08,9XAB@0C];[DAEF@00,5M67@08,9XAB@0C,5M67@08,1P23@04]]\n"))]
    #[test_case(Setup {
        rng: Some(Rng::Seeded{ seed: 123 }),
        battle_system: Some(BattleSystem::Dice { sides: 8 }),
        blocked_cells: Some(vec![2, 8, 0xA]),
        hand_candidates: Some([
            [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
            [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
            [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
        ]),
    } => using assert_eq("setup rng=seed(123) battle_system=dice(8) blocked_cells=[2,8,A] hand_candidates=[[1P23@04,5M67@08,9XAB@0C,DAEF@00,5M67@08];[5M67@08,1P23@04,DAEF@00,5M67@08,9XAB@0C];[DAEF@00,5M67@08,9XAB@0C,5M67@08,1P23@04]]\n")
    )]
    fn setup(input: Command) -> String {
        let mut out = String::new();
        input.serialize(&mut out).unwrap();
        out
    }

    #[test_case(PickHand { index: 0 } => using assert_eq("pick-hand index=0\n"))]
    #[test_case(PickHand { index: 1 } => using assert_eq("pick-hand index=1\n"))]
    #[test_case(PickHand { index: 2 } => using assert_eq("pick-hand index=2\n"))]
    fn pick_hand(input: Command) -> String {
        let mut out = String::new();
        input.serialize(&mut out).unwrap();
        out
    }

    #[test_case(PlaceCard { card: 0, cell: 0 } => using assert_eq("place-card card=0 cell=0\n"))]
    #[test_case(PlaceCard { card: 3, cell: 0xA } => using assert_eq("place-card card=3 cell=A\n"))]
    fn place_card(input: Command) -> String {
        let mut out = String::new();
        input.serialize(&mut out).unwrap();
        out
    }

    #[test_case(PickBattle { cell: 0 } => using assert_eq("pick-battle cell=0\n"))]
    #[test_case(PickBattle { cell: 0xA } => using assert_eq("pick-battle cell=A\n"))]
    fn pick_battle(input: Command) -> String {
        let mut out = String::new();
        input.serialize(&mut out).unwrap();
        out
    }
}
