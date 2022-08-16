use crate::{Card, CardType, HandCandidates, Seed};
use std::fmt::{Result, Write};

#[derive(Debug)]
pub(crate) enum Command {
    Quit,
    Setup {
        seed: Option<Seed>,
        blocked_cells: Option<Vec<u8>>,
        hand_candidates: Option<HandCandidates>,
    },
}

impl Command {
    pub(crate) fn serialize(self, out: &mut String) -> anyhow::Result<()> {
        match self {
            Command::Quit => out.write_str("quit")?,
            Command::Setup {
                seed,
                blocked_cells,
                hand_candidates,
            } => {
                out.write_str("setup")?;
                if let Some(seed) = seed {
                    write!(out, " seed={seed}")?;
                }
                if let Some(blocked_cells) = blocked_cells {
                    write!(out, " blocked_cells=")?;
                    write_list(out, ',', blocked_cells.iter(), |out, v| {
                        write!(out, "{v:X}")
                    })?;
                }
                if let Some(hand_candidates) = hand_candidates {
                    write!(out, " hand_candidates=")?;
                    write_list(out, ';', hand_candidates.iter(), |out, hand| {
                        write_list(out, ',', hand.iter(), |out, card| write_card(out, *card))
                    })?;
                }
            }
        }
        out.write_char('\n')?;

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
    write!(out, "{att:X}{typ}{phy:X}{mag:X}@{arr:X}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Arrows;
    use pretty_assertions::assert_eq;

    const C1P23_4: Card = Card::new(1, CardType::Physical, 2, 3, Arrows::new(4));
    const C5M67_8: Card = Card::new(5, CardType::Magical, 6, 7, Arrows::new(8));
    const C9XAB_C: Card = Card::new(9, CardType::Exploit, 0xA, 0xB, Arrows::new(0xC));
    const CDAEF_0: Card = Card::new(0xD, CardType::Assault, 0xE, 0xF, Arrows::new(0));

    #[test]
    fn setup() {
        use Command::*;

        let mut actual = String::new();
        for (input, expected) in [
            (
                Setup {
                    seed: None,
                    blocked_cells: None,
                    hand_candidates: None,
                },
                "setup\n",
            ),
            (
                Setup {
                    seed: Some(123),
                    blocked_cells: None,
                    hand_candidates: None,
                },
                "setup seed=123\n",
            ),
            (
                Setup {
                    seed: None,
                    blocked_cells: Some(vec![]),
                    hand_candidates: None,
                },
                "setup blocked_cells=[]\n",
            ),
            (
                Setup {
                    seed: None,
                    blocked_cells: Some(vec![2]),
                    hand_candidates: None,
                },
                "setup blocked_cells=[2]\n",
            ),
            (
                Setup {
                    seed: None,
                    blocked_cells: None,
                    hand_candidates: Some([
                        [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
                        [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
                        [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
                    ]),
                },
                "setup hand_candidates=[[1P23@4,5M67@8,9XAB@C,DAEF@0,5M67@8];[5M67@8,1P23@4,DAEF@0,5M67@8,9XAB@C];[DAEF@0,5M67@8,9XAB@C,5M67@8,1P23@4]]\n",
            ),
            (
                Setup {
                    seed: Some(123),
                    blocked_cells: Some(vec![2, 8, 0xA]),
                    hand_candidates: Some([
                        [C1P23_4, C5M67_8, C9XAB_C, CDAEF_0, C5M67_8],
                        [C5M67_8, C1P23_4, CDAEF_0, C5M67_8, C9XAB_C],
                        [CDAEF_0, C5M67_8, C9XAB_C, C5M67_8, C1P23_4],
                    ]),
                },
                "setup seed=123 blocked_cells=[2,8,A] hand_candidates=[[1P23@4,5M67@8,9XAB@C,DAEF@0,5M67@8];[5M67@8,1P23@4,DAEF@0,5M67@8,9XAB@C];[DAEF@0,5M67@8,9XAB@C,5M67@8,1P23@4]]\n",
            ),
        ] {
            dbg!((&input, &expected));
            actual.clear();
            input.serialize(&mut actual).unwrap();
            assert_eq!(expected, actual);
        }
    }
}
