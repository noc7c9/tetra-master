use crate::{
    Arrows, BattleSystem, BattleWinner, Battler, Card, CardType, Digit, Event, Hand,
    HandCandidates, Player,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::{char, multispace0, multispace1, one_of},
    combinator::{map, map_res, opt, verify},
    error::Error as NomError,
    multi::separated_list0,
    sequence::{delimited, preceded, terminated, tuple},
    IResult, Parser,
};

// TODO: replace this with a bespoke Error enum
pub type Error = nom::Err<NomError<String>>;

pub trait Response: Sized {
    fn deserialize(input: &str) -> Result<Self, Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorResponse {
    InvalidHandPick { hand: u8 },
    HandAlreadyPicked { hand: u8 },
    CellIsNotEmpty { cell: u8 },
    CardAlreadyPlayed { card: u8 },
    InvalidBattlePick { cell: u8 },
}

impl Response for ErrorResponse {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = error(i).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOk {
    pub battle_system: BattleSystem,
    pub blocked_cells: Vec<u8>,
    pub hand_candidates: HandCandidates,
}

impl Response for SetupOk {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = setup_ok(i).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRngNumbersOk {
    pub numbers_left: usize,
}

impl Response for PushRngNumbersOk {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = push_rng_numbers_ok(i).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickHandOk;

impl Response for PickHandOk {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = pick_hand_ok(i).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceCardOk {
    pub pick_battle: Vec<u8>,
    pub events: Vec<Event>,
}

impl Response for PlaceCardOk {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = place_card_ok(i).map_err(|e| e.to_owned())?;
        Ok(res)
    }
}

fn atom<'a, T>(
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    delimited(multispace0, inner, multispace0)
}

fn ident<'a>(value: &'static str) -> impl FnMut(&'a str) -> IResult<&'a str, &str> {
    atom(tag(value))
}

fn list<'a, T>(
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    delimited(char('('), atom(inner), char(')'))
}

fn array<'a, T>(
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    list(separated_list0(multispace1, inner))
}

fn array_verify<'a, T>(
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
    predicate: impl Fn(&Vec<T>) -> bool,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<T>> {
    verify(array(inner), predicate)
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

impl FromStrHex for usize {
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
        map(ident("original"), |_| BattleSystem::Original),
        map(preceded(ident("dice"), atom(hex_digit_1_n(2))), |sides| {
            BattleSystem::Dice { sides }
        }),
        map(ident("test"), |_| BattleSystem::Test),
    ))(i)
}

fn blocked_cells(i: &str) -> IResult<&str, Vec<u8>> {
    array_verify(hex_digit_1_n(1), |v| v.len() < 6)(i)
}

fn hand(i: &str) -> IResult<&str, Hand> {
    let (i, cards) = array_verify(card, |v| v.len() == crate::HAND_SIZE)(i)?;
    Ok((i, [cards[0], cards[1], cards[2], cards[3], cards[4]]))
}

fn hand_candidates(i: &str) -> IResult<&str, HandCandidates> {
    let (i, hands) = array_verify(hand, |v| v.len() == crate::HAND_CANDIDATES)(i)?;
    Ok((i, [hands[0], hands[1], hands[2]]))
}

fn response<'a, T>(
    name: &'static str,
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    terminated(list(preceded(ident(name), inner)), char('\n'))
}

fn prop<'a, T>(
    name: &'static str,
    inner: impl Parser<&'a str, T, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, T> {
    preceded(multispace0, list(preceded(ident(name), inner)))
}

fn error(i: &str) -> IResult<&str, ErrorResponse> {
    use ErrorResponse::*;

    fn invalid_hand_pick(i: &str) -> IResult<&str, ErrorResponse> {
        let (i, _) = ident("InvalidHandPick")(i)?;
        let (i, hand) = prop("hand", hex_digit_1_n(1))(i)?;
        Ok((i, InvalidHandPick { hand }))
    }

    fn already_picked_hand(i: &str) -> IResult<&str, ErrorResponse> {
        let (i, _) = ident("HandAlreadyPicked")(i)?;
        let (i, hand) = prop("hand", hex_digit_1_n(1))(i)?;
        Ok((i, HandAlreadyPicked { hand }))
    }

    fn cell_is_not_empty(i: &str) -> IResult<&str, ErrorResponse> {
        let (i, _) = ident("CellIsNotEmpty")(i)?;
        let (i, cell) = prop("cell", hex_digit_1_n(1))(i)?;
        Ok((i, CellIsNotEmpty { cell }))
    }

    fn card_already_played(i: &str) -> IResult<&str, ErrorResponse> {
        let (i, _) = ident("CardAlreadyPlayed")(i)?;
        let (i, card) = prop("card", hex_digit_1_n(1))(i)?;
        Ok((i, CardAlreadyPlayed { card }))
    }

    fn invalid_battle_pick(i: &str) -> IResult<&str, ErrorResponse> {
        let (i, _) = ident("InvalidBattlePick")(i)?;
        let (i, cell) = prop("cell", hex_digit_1_n(1))(i)?;
        Ok((i, InvalidBattlePick { cell }))
    }

    response("error", |i| {
        alt((
            invalid_hand_pick,
            already_picked_hand,
            cell_is_not_empty,
            card_already_played,
            invalid_battle_pick,
        ))(i)
    })(i)
}

fn setup_ok(i: &str) -> IResult<&str, SetupOk> {
    response("setup-ok", |i| {
        let (i, battle_system) = prop("battle-system", battle_system)(i)?;
        let (i, blocked_cells) = prop("blocked-cells", blocked_cells)(i)?;
        let (i, hand_candidates) = prop("hand-candidates", hand_candidates)(i)?;
        let setup_ok = SetupOk {
            battle_system,
            blocked_cells,
            hand_candidates,
        };
        Ok((i, setup_ok))
    })(i)
}

fn push_rng_numbers_ok(i: &str) -> IResult<&str, PushRngNumbersOk> {
    response("push-rng-numbers-ok", |i| {
        let (i, numbers_left) = prop("numbers-left", hex_digit_1_n(8))(i)?;
        let push_rng_numbers_ok = PushRngNumbersOk { numbers_left };
        Ok((i, push_rng_numbers_ok))
    })(i)
}

fn pick_hand_ok(i: &str) -> IResult<&str, PickHandOk> {
    response("pick-hand-ok", |i| Ok((i, PickHandOk)))(i)
}

fn place_card_ok(i: &str) -> IResult<&str, PlaceCardOk> {
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
        let (i, events) = prop("events", separated_list0(multispace1, event))(i)?;
        let (i, pick_battle) = opt(prop("pick-battle", array(hex_digit_1_n(1))))(i)?;
        let place_card_ok = PlaceCardOk {
            pick_battle: pick_battle.unwrap_or_default(),
            events,
        };
        Ok((i, place_card_ok))
    })(i)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{
        Arrows, BattleSystem, BattleWinner, Battler, Card, Digit, Hand, HandCandidates, Player,
    };

    fn assert_eq<T: PartialEq + std::fmt::Debug>(expected: T) -> impl Fn(T) {
        move |actual| pretty_assertions::assert_eq!(actual, expected)
    }

    // note: first arg is used to differentiate names that generate with the same name types
    #[test_case(0, "0P00_0" => using assert_eq(Card::physical(0, 0, 0, Arrows(0))))]
    #[test_case(0, "0M00_0" => using assert_eq(Card::magical(0, 0, 0, Arrows(0))))]
    #[test_case(0, "0X00_0" => using assert_eq(Card::exploit(0, 0, 0, Arrows(0))))]
    #[test_case(0, "0A00_0" => using assert_eq(Card::assault(0, 0, 0, Arrows(0))))]
    #[test_case(1, "0p00_0" => using assert_eq(Card::physical(0, 0, 0, Arrows(0))))]
    #[test_case(1, "0m00_0" => using assert_eq(Card::magical(0, 0, 0, Arrows(0))))]
    #[test_case(1, "0x00_0" => using assert_eq(Card::exploit(0, 0, 0, Arrows(0))))]
    #[test_case(1, "0a00_0" => using assert_eq(Card::assault(0, 0, 0, Arrows(0))))]
    #[test_case(1, "0B00_0" => panics)]
    // stats
    #[test_case(0, "1P23_0" => using assert_eq(Card::physical(1, 2, 3, Arrows(0))))]
    #[test_case(0, "aPbc_0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, Arrows(0))))]
    #[test_case(1, "APBC_0" => using assert_eq(Card::physical(0xa, 0xb, 0xc, Arrows(0))))]
    // arrows
    #[test_case(0, "0P00_1" => using assert_eq(Card::physical(0, 0, 0, Arrows(1))))]
    #[test_case(0, "0P00_F" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf))))]
    #[test_case(1, "0P00_f" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf))))]
    #[test_case(0, "0P00_00" => using assert_eq(Card::physical(0, 0, 0, Arrows(0))))]
    #[test_case(0, "0P00_0F" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf))))]
    #[test_case(1, "0P00_0f" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf))))]
    #[test_case(0, "0P00_f0" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf0))))]
    #[test_case(1, "0P00_F0" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xf0))))]
    #[test_case(0, "0P00_fF" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xff))))]
    #[test_case(1, "0P00_ff" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xff))))]
    #[test_case(2, "0P00_FF" => using assert_eq(Card::physical(0, 0, 0, Arrows(0xff))))]
    fn card_(_: u8, i: &str) -> Card {
        super::card(i).unwrap().1
    }

    const C0P00: Card = Card::physical(0, 0, 0, Arrows(0));
    const C1X23: Card = Card::exploit(1, 2, 3, Arrows(0x45));

    #[test_case("(0P00_0 0P00_0 0P00_0 0P00_0 1X23_45)"
        => using assert_eq([C0P00,C0P00,C0P00,C0P00,C1X23]))]
    #[test_case("(0P00_0 0P00_0 0P00_0 1X23_45 0P00_0)"
        => using assert_eq([C0P00,C0P00,C0P00,C1X23,C0P00]))]
    #[test_case("(0P00_0 0P00_0 1X23_45 0P00_0 0P00_0)"
        => using assert_eq([C0P00,C0P00,C1X23,C0P00,C0P00]))]
    #[test_case("(0P00_0 1X23_45 0P00_0 0P00_0 0P00_0)"
        => using assert_eq([C0P00,C1X23,C0P00,C0P00,C0P00]))]
    #[test_case("(1X23_45 0P00_0 0P00_0 0P00_0 0P00_0)"
        => using assert_eq([C1X23,C0P00,C0P00,C0P00,C0P00]))]
    #[test_case("(0P00_0 0P00_0 0P00_0 0P00_0)" => panics)]
    #[test_case("()" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn hand(i: &str) -> Hand {
        super::hand(i).unwrap().1
    }

    #[test_case(concat!("((0P00_0 0P00_0 0P00_0 0P00_0 1X23_45)",
                        " (0P00_0 0P00_0 1X23_45 0P00_0 0P00_0)",
                        " (1X23_45 0P00_0 0P00_0 0P00_0 0P00_0))")
        => using assert_eq([[C0P00,C0P00,C0P00,C0P00,C1X23], [C0P00,C0P00,C1X23,C0P00,C0P00],
                            [C1X23,C0P00,C0P00,C0P00,C0P00]]))]
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
    #[test_case("dice 4" => BattleSystem::Dice { sides: 4 })]
    #[test_case("dice A" => BattleSystem::Dice { sides: 10 })]
    #[test_case("dice 11" => BattleSystem::Dice { sides: 17 })]
    #[test_case("dice" => panics)]
    #[test_case("test" => BattleSystem::Test)]
    #[test_case("" => panics)]
    fn battle_system(i: &str) -> BattleSystem {
        super::battle_system(i).unwrap().1
    }

    #[test_case("(error InvalidHandPick (hand 1))\n" => ErrorResponse::InvalidHandPick { hand: 1 })]
    #[test_case("(error HandAlreadyPicked (hand 2))\n" => ErrorResponse::HandAlreadyPicked { hand: 2 })]
    #[test_case("(error CellIsNotEmpty (cell 2))\n" => ErrorResponse::CellIsNotEmpty { cell: 2 })]
    #[test_case("(error CardAlreadyPlayed (card 2))\n" => ErrorResponse::CardAlreadyPlayed { card: 2 })]
    #[test_case("(error InvalidBattlePick (cell 2))\n" => ErrorResponse::InvalidBattlePick { cell: 2 })]
    fn error(i: &str) -> ErrorResponse {
        Response::deserialize(i).unwrap()
    }

    const BLOCKED_CELLS: &str = "(2 3 F)";
    const HAND_CANDIDATES: &str = concat!(
        "((0P00_0 0P00_0 0P00_0 0P00_0 0P00_0)",
        " (0P00_0 0P00_0 0P00_0 0P00_0 0P00_0)",
        " (0P00_0 0P00_0 0P00_0 0P00_0 0P00_0))"
    );
    #[test_case(
        format!(concat!("(setup-ok (battle-system dice 9) (blocked-cells {})",
                                 " (hand-candidates {}))\n"), BLOCKED_CELLS, HAND_CANDIDATES)
        => SetupOk {
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
        format!("(setup-ok (battle-system dice 9) (blocked-cells {}) (hand-candidates {}))\n",
                BLOCKED_CELLS, HAND_CANDIDATES)
        => SetupOk {
            battle_system: BattleSystem::Dice { sides: 9 },
            blocked_cells: vec![2, 3, 0xf],
            hand_candidates: [
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
                [C0P00, C0P00, C0P00, C0P00, C0P00],
            ],
        }
    )]
    fn setup_ok(i: String) -> SetupOk {
        Response::deserialize(&i).unwrap()
    }

    #[test_case("(push-rng-numbers-ok (numbers-left 0))\n" => PushRngNumbersOk { numbers_left: 0 })]
    #[test_case("(push-rng-numbers-ok (numbers-left aBc123))\n"
        => PushRngNumbersOk { numbers_left: 0xABC123 })]
    fn push_rng_numbers_ok(i: &str) -> PushRngNumbersOk {
        Response::deserialize(i).unwrap()
    }

    #[test_case("(pick-hand-ok)\n" => PickHandOk)]
    fn pick_hand_ok(i: &str) -> PickHandOk {
        Response::deserialize(i).unwrap()
    }

    use Event::*;
    #[test_case("(place-card-ok (events))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![] })]
    #[test_case("(place-card-ok (events (next-turn player1)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![NextTurn { to: Player::P1 }] })]
    #[test_case("(place-card-ok (events (next-turn player2)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![NextTurn { to: Player::P2 }] })]
    #[test_case("(place-card-ok (events (flip 4)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![Flip { cell: 4 }] })]
    #[test_case("(place-card-ok (events (flip 2) (flip A) (flip 7)))\n"
        => PlaceCardOk { pick_battle: vec![],
            events: vec![Flip { cell: 2 }, Flip { cell: 0xA }, Flip { cell: 7 }] })]
    #[test_case("(place-card-ok (events (combo-flip A)))\n"
        => PlaceCardOk { events: vec![ComboFlip { cell: 0xA }], pick_battle: vec![] })]
    #[test_case("(place-card-ok (events (combo-flip 3) (combo-flip C)))\n"
        => PlaceCardOk { pick_battle: vec![],
            events: vec![ComboFlip { cell: 3 }, ComboFlip { cell: 0xC }] })]
    #[test_case("(place-card-ok (events (battle (1 A D 2) (8 m 3 Cd) attacker)))\n"
        => PlaceCardOk { pick_battle: vec![],
            events: vec![Battle {
                attacker: Battler { cell: 1, digit: Digit::Attack, value: 0xD, roll: 2 },
                defender: Battler { cell: 8, digit: Digit::MagicalDefense, value: 3, roll: 0xCD },
                winner: BattleWinner::Attacker,
            }] })]
    #[test_case("(place-card-ok (events (game-over player1)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![GameOver { winner: Some(Player::P1) }] })]
    #[test_case("(place-card-ok (events (game-over player2)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![GameOver { winner: Some(Player::P2) }] })]
    #[test_case("(place-card-ok (events (game-over draw)))\n"
        => PlaceCardOk { pick_battle: vec![], events: vec![GameOver { winner: None }] })]
    #[test_case("(place-card-ok (events) (pick-battle (2 3 4)))\n"
        => PlaceCardOk { pick_battle: vec![2, 3, 4], events: vec![] })]
    fn place_card_ok(i: &str) -> PlaceCardOk {
        Response::deserialize(i).unwrap()
    }
}
