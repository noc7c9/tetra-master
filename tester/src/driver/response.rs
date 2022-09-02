use crate::{Arrows, BattleSystem, Card, CardType, HandCandidate, HandCandidates, Player, Seed};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while_m_n},
    character::complete::{char, multispace0, multispace1, one_of},
    combinator::{map, map_res, opt, verify},
    error::Error,
    multi::separated_list0,
    sequence::{delimited, preceded, terminated, tuple},
    IResult, Parser,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Response {
    SetupOk {
        seed: Option<Seed>,
        battle_system: BattleSystem,
        blocked_cells: Vec<u8>,
        hand_candidates: HandCandidates,
    },
    PickHandOk,
    PickHandErr {
        reason: String,
    },
    PlaceCardOk {
        events: Vec<Event>,
    },
    PlaceCardPickBattle {
        choices: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    NextTurn {
        to: Player,
    },
    Flip {
        cell: u8,
    },
    ComboFlip {
        cell: u8,
    },
    Battle {
        attacker: Battler,
        defender: Battler,
        winner: BattleWinner,
    },
    GameOver {
        winner: Option<Player>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Battler {
    pub cell: u8,
    pub digit: Digit,
    pub value: u8,
    pub roll: u8,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum Digit {
    Attack,
    PhysicalDefense,
    MagicalDefense,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum BattleWinner {
    Attacker,
    Defender,
    None,
}

impl Response {
    pub(crate) fn deserialize(i: &str) -> anyhow::Result<Self> {
        let (_, res) = alt((
            setup_ok,
            pick_hand_ok,
            pick_hand_err,
            place_card_ok,
            place_card_pick_battle,
        ))(i)
        .map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

fn atom<'a, T>(
    inner: impl Parser<&'a str, T, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    delimited(multispace0, inner, multispace0)
}

fn ident<'a>(value: &'static str) -> impl FnMut(&'a str) -> IResult<&'a str, &str> {
    atom(tag(value))
}

fn list<'a, T>(
    inner: impl Parser<&'a str, T, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    delimited(char('('), atom(inner), char(')'))
}

fn array<'a, T>(
    inner: impl Parser<&'a str, T, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    list(separated_list0(multispace1, inner))
}

fn array_verify<'a, T>(
    inner: impl Parser<&'a str, T, Error<&'a str>>,
    predicate: impl Fn(&Vec<T>) -> bool,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    verify(array(inner), predicate)
}

fn string(i: &str) -> IResult<&str, String> {
    let (i, string) = delimited(char('"'), is_not("\""), char('"'))(i)?;
    Ok((i, string.into()))
}

fn hex_digit_1_n<'a, T: FromStrHex>(n: usize) -> impl FnMut(&'a str) -> IResult<&str, T> {
    map_res(take_while_m_n(1, n, |c: char| c.is_ascii_hexdigit()), |i| {
        T::from_str_hex(i)
    })
}

trait FromStrHex: Sized {
    fn from_str_hex(s: &str) -> Result<Self, std::num::ParseIntError>;
}

impl FromStrHex for u8 {
    fn from_str_hex(s: &str) -> Result<Self, std::num::ParseIntError> {
        Self::from_str_radix(s, 16)
    }
}

impl FromStrHex for u64 {
    fn from_str_hex(s: &str) -> Result<Self, std::num::ParseIntError> {
        Self::from_str_radix(s, 16)
    }
}

fn player(i: &str) -> IResult<&str, Player> {
    alt((
        map(ident("player1"), |_| Player::P1),
        map(ident("player2"), |_| Player::P2),
    ))(i)
}

fn card(i: &str) -> IResult<&str, Card> {
    let (i, attack) = hex_digit_1_n(1)(i)?;
    let (i, card_type) = map(one_of("PMXApmxa"), |ch| match ch {
        'P' | 'p' => CardType::Physical,
        'M' | 'm' => CardType::Magical,
        'X' | 'x' => CardType::Exploit,
        'A' | 'a' => CardType::Assault,
        _ => unreachable!(),
    })(i)?;
    let (i, physical_defense) = hex_digit_1_n(1)(i)?;
    let (i, magical_defense) = hex_digit_1_n(1)(i)?;
    let (i, _) = char('_')(i)?;
    let (i, arrows) = map(hex_digit_1_n(2), Arrows)(i)?;
    Ok((
        i,
        Card {
            attack,
            card_type,
            physical_defense,
            magical_defense,
            arrows,
        },
    ))
}

fn battle_system(i: &str) -> IResult<&str, BattleSystem> {
    alt((
        map(ident("original-approx"), |_| BattleSystem::OriginalApprox),
        map(ident("original"), |_| BattleSystem::Original),
        map(preceded(ident("dice"), atom(hex_digit_1_n(2))), |sides| {
            BattleSystem::Dice { sides }
        }),
    ))(i)
}

fn blocked_cells(i: &str) -> IResult<&str, Vec<u8>> {
    array_verify(hex_digit_1_n(1), |v| v.len() < 6)(i)
}

fn hand(i: &str) -> IResult<&str, HandCandidate> {
    let (i, cards) = array_verify(card, |v| v.len() == crate::HAND_SIZE)(i)?;
    Ok((i, [cards[0], cards[1], cards[2], cards[3], cards[4]]))
}

fn hand_candidates(i: &str) -> IResult<&str, HandCandidates> {
    let (i, hands) = array_verify(hand, |v| v.len() == crate::HAND_CANDIDATES)(i)?;
    Ok((i, [hands[0], hands[1], hands[2]]))
}

fn response<'a>(
    name: &'static str,
    inner: impl Parser<&'a str, Response, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, Response> {
    terminated(list(preceded(ident(name), inner)), char('\n'))
}

fn prop<'a, T>(
    name: &'static str,
    inner: impl Parser<&'a str, T, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    preceded(multispace0, list(preceded(ident(name), inner)))
}

fn setup_ok(i: &str) -> IResult<&str, Response> {
    response("setup-ok", |i| {
        let (i, seed) = opt(prop("seed", hex_digit_1_n(16)))(i)?;
        let (i, battle_system) = prop("battle-system", battle_system)(i)?;
        let (i, blocked_cells) = prop("blocked-cells", blocked_cells)(i)?;
        let (i, hand_candidates) = prop("hand-candidates", hand_candidates)(i)?;
        let response = Response::SetupOk {
            seed,
            battle_system,
            blocked_cells,
            hand_candidates,
        };
        Ok((i, response))
    })(i)
}

fn pick_hand_ok(i: &str) -> IResult<&str, Response> {
    response("pick-hand-ok", |i| Ok((i, Response::PickHandOk)))(i)
}

fn pick_hand_err(i: &str) -> IResult<&str, Response> {
    response("pick-hand-err", |i| {
        let (i, reason) = prop("reason", string)(i)?;
        Ok((i, Response::PickHandErr { reason }))
    })(i)
}

fn place_card_ok(i: &str) -> IResult<&str, Response> {
    fn next_turn(i: &str) -> IResult<&str, Event> {
        let (i, to) = prop("next-turn", player)(i)?;
        Ok((i, Event::NextTurn { to }))
    }

    fn flip(i: &str) -> IResult<&str, Event> {
        let (i, cell) = prop("flip", hex_digit_1_n(1))(i)?;
        Ok((i, Event::Flip { cell }))
    }

    fn combo_flip(i: &str) -> IResult<&str, Event> {
        let (i, cell) = prop("combo-flip", hex_digit_1_n(1))(i)?;
        Ok((i, Event::ComboFlip { cell }))
    }

    fn battle(i: &str) -> IResult<&str, Event> {
        fn digit(i: &str) -> IResult<&str, Digit> {
            map(one_of("PMApmx"), |ch| match ch {
                'A' | 'a' => Digit::Attack,
                'P' | 'p' => Digit::PhysicalDefense,
                'M' | 'm' => Digit::MagicalDefense,
                _ => unreachable!(),
            })(i)
        }

        fn battler(i: &str) -> IResult<&str, Battler> {
            let (i, (cell, digit, value, roll)) = list(tuple((
                atom(hex_digit_1_n(1)),
                atom(digit),
                atom(hex_digit_1_n(1)),
                atom(hex_digit_1_n(2)),
            )))(i)?;
            let battler = Battler {
                cell,
                digit,
                value,
                roll,
            };
            Ok((i, battler))
        }

        fn winner(i: &str) -> IResult<&str, BattleWinner> {
            alt((
                map(tag("attacker"), |_| BattleWinner::Attacker),
                map(tag("defender"), |_| BattleWinner::Defender),
                map(tag("none"), |_| BattleWinner::None),
            ))(i)
        }

        let (i, (attacker, defender, winner)) = prop(
            "battle",
            tuple((atom(battler), atom(battler), atom(winner))),
        )(i)?;
        let battle = Event::Battle {
            attacker,
            defender,
            winner,
        };
        Ok((i, battle))
    }

    fn game_over(i: &str) -> IResult<&str, Event> {
        let (i, winner) = prop(
            "game-over",
            alt((map(player, Some), map(tag("draw"), |_| None))),
        )(i)?;
        Ok((i, Event::GameOver { winner }))
    }

    fn event(i: &str) -> IResult<&str, Event> {
        alt((next_turn, flip, combo_flip, battle, game_over))(i)
    }

    response("place-card-ok", |i| {
        let (i, events) = atom(separated_list0(multispace1, event))(i)?;
        Ok((i, Response::PlaceCardOk { events }))
    })(i)
}

fn place_card_pick_battle(i: &str) -> IResult<&str, Response> {
    response("place-card-pick-battle", |i| {
        let (i, choices) = prop("choices", array(hex_digit_1_n(1)))(i)?;
        Ok((i, Response::PlaceCardPickBattle { choices }))
    })(i)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{
        BattleSystem, BattleWinner, Battler, Digit,
        Event::*,
        Response::{self, *},
    };
    use crate::{Card, HandCandidate, HandCandidates, Player};

    fn assert_eq<T: PartialEq + std::fmt::Debug>(expected: T) -> impl Fn(T) {
        move |actual| pretty_assertions::assert_eq!(actual, expected)
    }

    // note: first arg is used to differentiate names that generate with the same name types
    #[test_case(0, "0P00_0" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(0, "0M00_0" => using assert_eq(Card::magical(0, 0, 0, 0)))]
    #[test_case(0, "0X00_0" => using assert_eq(Card::exploit(0, 0, 0, 0)))]
    #[test_case(0, "0A00_0" => using assert_eq(Card::assault(0, 0, 0, 0)))]
    #[test_case(1, "0p00_0" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(1, "0m00_0" => using assert_eq(Card::magical(0, 0, 0, 0)))]
    #[test_case(1, "0x00_0" => using assert_eq(Card::exploit(0, 0, 0, 0)))]
    #[test_case(1, "0a00_0" => using assert_eq(Card::assault(0, 0, 0, 0)))]
    #[test_case(1, "0B00_0" => panics)]
    // stats
    #[test_case(0, "1P23_0" => using assert_eq(Card::physical(1, 2, 3, 0)))]
    #[test_case(0, "aPbc_0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, 0)))]
    #[test_case(1, "APBC_0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, 0)))]
    // arrows
    #[test_case(0, "0P00_1" => using assert_eq(Card::physical(0, 0, 0, 1)))]
    #[test_case(0, "0P00_F" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(1, "0P00_f" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(0, "0P00_00" => using assert_eq(Card::physical(0, 0, 0, 0)))]
    #[test_case(0, "0P00_0F" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(1, "0P00_0f" => using assert_eq(Card::physical(0, 0, 0, 0xf)))]
    #[test_case(0, "0P00_f0" => using assert_eq(Card::physical(0, 0, 0, 0xf0)))]
    #[test_case(1, "0P00_F0" => using assert_eq(Card::physical(0, 0, 0, 0xf0)))]
    #[test_case(0, "0P00_fF" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    #[test_case(1, "0P00_ff" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    #[test_case(2, "0P00_FF" => using assert_eq(Card::physical(0, 0, 0, 0xff)))]
    fn card_(_: u8, i: &str) -> Card {
        super::card(i).unwrap().1
    }

    const C0P00: Card = Card::physical(0, 0, 0, 0);
    const C1X23: Card = Card::exploit(1, 2, 3, 0x45);

    #[test_case("(0P00_0 0P00_0 0P00_0 0P00_0 1X23_45)" => using assert_eq([C0P00,C0P00,C0P00,C0P00,C1X23]))]
    #[test_case("(0P00_0 0P00_0 0P00_0 1X23_45 0P00_0)" => using assert_eq([C0P00,C0P00,C0P00,C1X23,C0P00]))]
    #[test_case("(0P00_0 0P00_0 1X23_45 0P00_0 0P00_0)" => using assert_eq([C0P00,C0P00,C1X23,C0P00,C0P00]))]
    #[test_case("(0P00_0 1X23_45 0P00_0 0P00_0 0P00_0)" => using assert_eq([C0P00,C1X23,C0P00,C0P00,C0P00]))]
    #[test_case("(1X23_45 0P00_0 0P00_0 0P00_0 0P00_0)" => using assert_eq([C1X23,C0P00,C0P00,C0P00,C0P00]))]
    #[test_case("(0P00_0 0P00_0 0P00_0 0P00_0)" => panics)]
    #[test_case("()" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn hand(i: &str) -> HandCandidate {
        super::hand(i).unwrap().1
    }

    #[test_case("((0P00_0 0P00_0 0P00_0 0P00_0 1X23_45) (0P00_0 0P00_0 1X23_45 0P00_0 0P00_0) (1X23_45 0P00_0 0P00_0 0P00_0 0P00_0))" => using assert_eq([[C0P00,C0P00,C0P00,C0P00,C1X23], [C0P00,C0P00,C1X23,C0P00,C0P00], [C1X23,C0P00,C0P00,C0P00,C0P00]]))]
    #[test_case("()" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn hand_candidates(i: &str) -> HandCandidates {
        super::hand_candidates(i).unwrap().1
    }

    #[test_case("()" => Vec::<u8>::new())]
    #[test_case("(1)" => vec![1])]
    #[test_case("(2 a B F)" => vec![2,0xa,0xb,0xf])]
    #[test_case("(0 0 0 0 0 0 0)" => panics)]
    #[test_case("(2a)" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn blocked_cells(i: &str) -> Vec<u8> {
        super::blocked_cells(i).unwrap().1
    }

    #[test_case("original" => BattleSystem::Original)]
    #[test_case("original-approx" => BattleSystem::OriginalApprox)]
    #[test_case("dice 4" => BattleSystem::Dice { sides: 4 })]
    #[test_case("dice A" => BattleSystem::Dice { sides: 10 })]
    #[test_case("dice 11" => BattleSystem::Dice { sides: 17 })]
    #[test_case("dice" => panics)]
    #[test_case("" => panics)]
    fn battle_system(i: &str) -> BattleSystem {
        super::battle_system(i).unwrap().1
    }

    const BLOCKED_CELLS: &str = "(2 3 F)";
    const HAND_CANDIDATES: &str = "((0P00_0 0P00_0 0P00_0 0P00_0 0P00_0) (0P00_0 0P00_0 0P00_0 0P00_0 0P00_0) (0P00_0 0P00_0 0P00_0 0P00_0 0P00_0))";
    #[test_case(
        format!("(setup-ok (seed 7B) (battle-system dice 9) (blocked-cells {BLOCKED_CELLS}) (hand-candidates {HAND_CANDIDATES}))\n")
        => SetupOk {
            seed: Some(123),
            battle_system: BattleSystem::Dice { sides: 9 },
            blocked_cells: vec![2, 3, 0xf],
            hand_candidates: [
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
            ],
        }
    )]
    #[test_case(
        format!("(setup-ok (battle-system dice 9) (blocked-cells {BLOCKED_CELLS}) (hand-candidates {HAND_CANDIDATES}))\n")
        => SetupOk {
            seed: None,
            battle_system: BattleSystem::Dice { sides: 9 },
            blocked_cells: vec![2, 3, 0xf],
            hand_candidates: [
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
            ],
        }
    )]
    fn setup_ok(i: String) -> Response {
        Response::deserialize(&i).unwrap()
    }

    #[test_case("(pick-hand-ok)\n" => PickHandOk)]
    fn pick_hand_ok(i: &str) -> Response {
        Response::deserialize(i).unwrap()
    }

    #[test_case("(pick-hand-err (reason \"oneword\"))\n" => PickHandErr { reason: "oneword".into() })]
    #[test_case("(pick-hand-err (reason \"multiple words\"))\n" => PickHandErr { reason: "multiple words".into() })]
    #[test_case("(pick-hand-err (reason \"escaped \\\" quote\"))\n" => panics)]
    fn pick_hand_err(i: &str) -> Response {
        Response::deserialize(i).unwrap()
    }

    #[test_case("(place-card-ok)\n" => PlaceCardOk { events: vec![] })]
    #[test_case("(place-card-ok (next-turn player1))\n" =>
        PlaceCardOk { events: vec![NextTurn { to: Player::P1 }] })]
    #[test_case("(place-card-ok (next-turn player2))\n" =>
        PlaceCardOk { events: vec![NextTurn { to: Player::P2 }] })]
    #[test_case("(place-card-ok (flip 4))\n" => PlaceCardOk { events: vec![Flip { cell: 4 }] })]
    #[test_case("(place-card-ok (flip 2) (flip A) (flip 7))\n" => PlaceCardOk { events: vec![
        Flip { cell: 2 }, Flip { cell: 0xA }, Flip { cell: 7 }] })]
    #[test_case("(place-card-ok (combo-flip A))\n" => PlaceCardOk { events: vec![ComboFlip { cell: 0xA }] })]
    #[test_case("(place-card-ok (combo-flip 3) (combo-flip C))\n" => PlaceCardOk { events: vec![
        ComboFlip { cell: 3 }, ComboFlip { cell: 0xC }] })]
    #[test_case("(place-card-ok (battle (1 A D 2) (8 m 3 Cd) attacker))\n" =>
        PlaceCardOk { events: vec![Battle {
            attacker: Battler { cell: 1, digit: Digit::Attack, value: 0xD, roll: 2 },
            defender: Battler { cell: 8, digit: Digit::MagicalDefense, value: 3, roll: 0xCD },
            winner: BattleWinner::Attacker,
        }] })]
    #[test_case("(place-card-ok (game-over player1))\n" => PlaceCardOk { events: vec![GameOver { winner: Some(Player::P1) }] })]
    #[test_case("(place-card-ok (game-over player2))\n" => PlaceCardOk { events: vec![GameOver { winner: Some(Player::P2) }] })]
    #[test_case("(place-card-ok (game-over draw))\n" => PlaceCardOk { events: vec![GameOver { winner: None }] })]
    fn place_card_ok(i: &str) -> Response {
        Response::deserialize(i).unwrap()
    }

    #[test_case("(place-card-pick-battle (choices (2 3 4)))\n"
        => PlaceCardPickBattle { choices: vec![2, 3, 4] })]
    fn place_card_pick_battle(i: &str) -> Response {
        Response::deserialize(i).unwrap()
    }
}
