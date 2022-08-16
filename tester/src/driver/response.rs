use crate::{Arrows, Card, CardType, HandCandidate, HandCandidates, Seed};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    character::complete::{char, one_of, u64},
    combinator::{map_res, verify},
    error::Error,
    multi::separated_list0,
    sequence::{delimited, preceded, tuple},
    IResult, Parser,
};

#[derive(Debug, PartialEq)]
pub(crate) enum Response {
    SetupOk {
        seed: Seed,
        blocked_cells: Vec<u8>,
        hand_candidates: HandCandidates,
    },
}

impl Response {
    pub(crate) fn deserialize(input: &str) -> anyhow::Result<Self> {
        let (_, res) = setup_ok(input).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

fn list<'a, T>(
    delimiter: char,
    item: impl Parser<&'a str, T, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    delimited(char('['), separated_list0(char(delimiter), item), char(']'))
}

fn list_of_length<'a, T>(
    delimiter: char,
    item: impl Parser<&'a str, T, Error<&'a str>>,
    len: usize,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    verify(list(delimiter, item), move |v: &Vec<T>| v.len() == len)
}

fn hex_digits<'a>(max: usize) -> impl FnMut(&'a str) -> IResult<&str, u8> {
    map_res(
        take_while_m_n(1, max, |c: char| c.is_ascii_hexdigit()),
        |input| u8::from_str_radix(input, 16),
    )
}

fn card_full(input: &str) -> IResult<&str, Card> {
    let (input, (attack, card_type, physical_defense, magical_defense, _, arrow)) = tuple((
        hex_digits(1),
        one_of("PMXA"),
        hex_digits(1),
        hex_digits(1),
        char('@'),
        hex_digits(2),
    ))(input)?;
    let card_type = match card_type {
        'P' => CardType::Physical,
        'M' => CardType::Magical,
        'X' => CardType::Exploit,
        'A' => CardType::Assault,
        _ => unreachable!(),
    };
    let arrows = Arrows(arrow);
    Ok((
        input,
        Card {
            attack,
            card_type,
            physical_defense,
            magical_defense,
            arrows,
        },
    ))
}

fn hand_candidate(input: &str) -> IResult<&str, HandCandidate> {
    let (input, cards) = list_of_length(',', card_full, crate::HAND_SIZE)(input)?;
    Ok((input, [cards[0], cards[1], cards[2], cards[3], cards[4]]))
}

fn hand_candidates(input: &str) -> IResult<&str, HandCandidates> {
    let (input, hands) = list_of_length(';', hand_candidate, crate::HAND_CANDIDATES)(input)?;
    Ok((input, [hands[0], hands[1], hands[2]]))
}

fn setup_ok(input: &str) -> IResult<&str, Response> {
    let (input, _) = tag("setup-ok")(input)?;

    let (input, seed) = preceded(tag(" seed="), u64)(input)?;
    let (input, blocked_cells) = preceded(tag(" blocked_cells="), list(',', hex_digits(1)))(input)?;
    let (input, hand_candidates) = preceded(tag(" hand_candidates="), hand_candidates)(input)?;

    Ok((
        input,
        Response::SetupOk {
            seed,
            blocked_cells,
            hand_candidates,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const C0P00: Card = Card::new(0, CardType::Physical, 0, 0, Arrows::new(0));
    const C1X23: Card = Card::new(1, CardType::Exploit, 2, 3, Arrows::new(0x45));

    #[test]
    fn setup_ok() {
        use Response::*;

        for (input, expected) in [
            (
                "setup-ok seed=123 blocked_cells=[2,3,f] hand_candidates=[[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0]]\n",
                SetupOk {
                    seed: 123,
                    blocked_cells: vec![2, 3, 0xf],
                    hand_candidates: [
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                    ],
                },
            ),
            (
                "setup-ok seed=1 blocked_cells=[] hand_candidates=[[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0]]\n",
                SetupOk {
                    seed: 1,
                    blocked_cells: vec![],
                    hand_candidates: [
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                    ],
                },
            ),
            (
                "setup-ok seed=1 blocked_cells=[] hand_candidates=[[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,1X23@45,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0]]\n",
                SetupOk {
                    seed: 1,
                    blocked_cells: vec![],
                    hand_candidates: [
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                        [C0P00, C0P00, C0P00, C1X23, C0P00],
                        [C0P00, C0P00, C0P00, C0P00, C0P00],
                    ],
                },
            ),
        ] {
            dbg!((&input, &expected));
            let actual = Response::deserialize(input).unwrap();
            assert_eq!(expected, actual);
        }
    }
}
