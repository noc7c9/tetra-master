use crate::{Arrows, Card, CardType, HandCandidate, HandCandidates, Seed};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while_m_n},
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
    PickHandOk,
    PickHandErr {
        reason: String,
    },
}

impl Response {
    pub(crate) fn deserialize(input: &str) -> anyhow::Result<Self> {
        let (_, res) =
            alt((setup_ok, pick_hand_ok, pick_hand_err))(input).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

fn string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"'))(input)
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

fn card(input: &str) -> IResult<&str, Card> {
    let (input, (attack, card_type, physical_defense, magical_defense, _, arrow)) = tuple((
        hex_digits(1),
        one_of("PMXApmxa"),
        hex_digits(1),
        hex_digits(1),
        char('@'),
        hex_digits(2),
    ))(input)?;
    let card_type = match card_type {
        'P' | 'p' => CardType::Physical,
        'M' | 'm' => CardType::Magical,
        'X' | 'x' => CardType::Exploit,
        'A' | 'a' => CardType::Assault,
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

fn blocked_cells(input: &str) -> IResult<&str, Vec<u8>> {
    verify(list(',', hex_digits(1)), |v: &Vec<_>| v.len() < 6)(input)
}

fn hand_candidate(input: &str) -> IResult<&str, HandCandidate> {
    let (input, cards) = list_of_length(',', card, crate::HAND_SIZE)(input)?;
    Ok((input, [cards[0], cards[1], cards[2], cards[3], cards[4]]))
}

fn hand_candidates(input: &str) -> IResult<&str, HandCandidates> {
    let (input, hands) = list_of_length(';', hand_candidate, crate::HAND_CANDIDATES)(input)?;
    Ok((input, [hands[0], hands[1], hands[2]]))
}

fn setup_ok(input: &str) -> IResult<&str, Response> {
    let (input, _) = tag("setup-ok")(input)?;

    let (input, seed) = preceded(tag(" seed="), u64)(input)?;
    let (input, blocked_cells) = preceded(tag(" blocked_cells="), blocked_cells)(input)?;
    let (input, hand_candidates) = preceded(tag(" hand_candidates="), hand_candidates)(input)?;

    let (input, _) = tag("\n")(input)?;

    Ok((
        input,
        Response::SetupOk {
            seed,
            blocked_cells,
            hand_candidates,
        },
    ))
}

fn pick_hand_ok(input: &str) -> IResult<&str, Response> {
    let (input, _) = tag("pick-hand-ok")(input)?;
    let (input, _) = tag("\n")(input)?;
    Ok((input, Response::PickHandOk))
}

fn pick_hand_err(input: &str) -> IResult<&str, Response> {
    let (input, _) = tag("pick-hand-err")(input)?;
    let (input, reason) = preceded(tag(" reason="), string)(input)?;
    let (input, _) = tag("\n")(input)?;
    Ok((
        input,
        Response::PickHandErr {
            reason: reason.into(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::Response::{self, *};
    use crate::{Card, HandCandidate};

    fn assert_eq<T: PartialEq + std::fmt::Debug>(expected: T) -> impl Fn(T) {
        move |actual| pretty_assertions::assert_eq!(actual, expected)
    }

    // note: first arg is used to differentiate names that generate with the same name types
    #[test_case(0, "0P00@0" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(0, "0M00@0" => using assert_eq(Card::magical(0, 0, 0, 0)))]
    #[test_case(0, "0X00@0" => using assert_eq(Card::exploit(0, 0, 0, 0)))]
    #[test_case(0, "0A00@0" => using assert_eq(Card::assault(0, 0, 0, 0)))]
    #[test_case(1, "0p00@0" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(1, "0m00@0" => using assert_eq(Card::magical(0, 0, 0, 0)))]
    #[test_case(1, "0x00@0" => using assert_eq(Card::exploit(0, 0, 0, 0)))]
    #[test_case(1, "0a00@0" => using assert_eq(Card::assault(0, 0, 0, 0)))]
    #[test_case(1, "0B00@0" => panics)]
    // stats
    #[test_case(0, "1P23@0" => using assert_eq(Card::physical(1, 2, 3, 0)))]
    #[test_case(0, "aPbc@0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, 0)))]
    #[test_case(1, "APBC@0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, 0)))]
    // arrows
    #[test_case(0, "0P00@1" => using assert_eq(Card::physical(0, 0, 0, 1)))]
    #[test_case(0, "0P00@F" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(1, "0P00@f" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(0, "0P00@00" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(0, "0P00@0F" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(1, "0P00@0f" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(0, "0P00@f0" => using assert_eq(Card::physical(0, 0, 0, 0xf0)))]
    #[test_case(1, "0P00@F0" => using assert_eq(Card::physical(0, 0, 0, 0xf0)))]
    #[test_case(0, "0P00@fF" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    #[test_case(1, "0P00@ff" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    #[test_case(2, "0P00@FF" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    fn card_(_: u8, input: &str) -> Card {
        super::card(input).unwrap().1
    }

    const C0P00: Card = Card::physical(0, 0, 0, 0);
    const C1X23: Card = Card::exploit(1, 2, 3, 0x45);

    #[test_case("[0P00@0,0P00@0,0P00@0,0P00@0,1X23@45]" => using assert_eq([C0P00,C0P00,C0P00,C0P00,C1X23]))]
    #[test_case("[0P00@0,0P00@0,0P00@0,1X23@45,0P00@0]" => using assert_eq([C0P00,C0P00,C0P00,C1X23,C0P00]))]
    #[test_case("[0P00@0,0P00@0,1X23@45,0P00@0,0P00@0]" => using assert_eq([C0P00,C0P00,C1X23,C0P00,C0P00]))]
    #[test_case("[0P00@0,1X23@45,0P00@0,0P00@0,0P00@0]" => using assert_eq([C0P00,C1X23,C0P00,C0P00,C0P00]))]
    #[test_case("[1X23@45,0P00@0,0P00@0,0P00@0,0P00@0]" => using assert_eq([C1X23,C0P00,C0P00,C0P00,C0P00]))]
    #[test_case("[0P00@0,0P00@0,0P00@0,0P00@0]" => panics)]
    #[test_case("[]" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn hand_candidate(input: &str) -> HandCandidate {
        super::hand_candidate(input).unwrap().1
    }

    #[test_case("[]" => Vec::<u8>::new())]
    #[test_case("[1]" => vec![1])]
    #[test_case("[2,a,B,F]" => vec![2,0xa,0xb,0xf])]
    #[test_case("[0,0,0,0,0,0,0]" => panics)]
    #[test_case("[2a]" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn blocked_cells(input: &str) -> Vec<u8> {
        super::blocked_cells(input).unwrap().1
    }

    const BLOCKED_CELLS: &str = "[2,3,F]";
    const HAND_CANDIDATES: &str = "[[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0];[0P00@0,0P00@0,0P00@0,0P00@0,0P00@0]]";
    #[test_case(
        format!("setup-ok seed=123 blocked_cells={BLOCKED_CELLS} hand_candidates={HAND_CANDIDATES}\n")
        ; "seed blocked_cells hand_candidates"
    )]
    // #[test_case(
    //     format!("setup-ok blocked_cells={BLOCKED_CELLS} seed=123 hand_candidates={HAND_CANDIDATES}\n")
    //     ; "blocked_cells seed hand_candidates"
    // )]
    // #[test_case(
    //     format!("setup-ok hand_candidates={HAND_CANDIDATES} seed=123 blocked_cells={BLOCKED_CELLS}\n")
    //     ; "hand_candidates seed blocked_cells"
    // )]
    // #[test_case(
    //     format!("setup-ok hand_candidates={HAND_CANDIDATES} blocked_cells={BLOCKED_CELLS} seed=123\n")
    //     ; "hand_candidates blocked_cells seed"
    // )]
    fn setup_ok(input: String) {
        let actual = Response::deserialize(&input).unwrap();
        let expected = SetupOk {
            seed: 123,
            blocked_cells: vec![2, 3, 0xf],
            hand_candidates: [
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
            ],
        };
        assert_eq!(expected, actual);
    }

    #[test_case("pick-hand-ok\n" => PickHandOk)]
    fn pick_hand_ok(input: &str) -> Response {
        Response::deserialize(input).unwrap()
    }

    #[test_case("pick-hand-err reason=\"oneword\"\n" => PickHandErr { reason: "oneword".into() })]
    #[test_case("pick-hand-err reason=\"multiple words\"\n" => PickHandErr { reason: "multiple words".into() })]
    #[test_case("pick-hand-err reason=\"escaped \\\" quote\"\n" => panics)]
    fn pick_hand_err(input: &str) -> Response {
        Response::deserialize(input).unwrap()
    }
}
