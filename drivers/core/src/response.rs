use crate::{
    Arrows, BattleSystem, BattleWinner, Battler, BoardCells, Card, CardType, Digit, Event, Hand,
    Player,
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
    pub blocked_cells: BoardCells,
    pub hand_red: Hand,
    pub hand_blue: Hand,
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
pub struct PlayOk {
    pub pick_battle: BoardCells,
    pub events: Vec<Event>,
}

impl Response for PlayOk {
    fn deserialize(i: &str) -> Result<Self, Error> {
        let (_, res) = play_ok(i).map_err(|e| e.to_owned())?;
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
        map(ident("blue"), |_| Player::Blue),
        map(ident("red"), |_| Player::Red),
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

fn blocked_cells(i: &str) -> IResult<&str, BoardCells> {
    let (i, cells) = array_verify(hex_digit_1_n(1), |v| v.len() < 6)(i)?;
    Ok((i, cells.into()))
}

fn hand(i: &str) -> IResult<&str, Hand> {
    let (i, cards) = array_verify(card, |v| v.len() == crate::HAND_SIZE)(i)?;
    Ok((i, [cards[0], cards[1], cards[2], cards[3], cards[4]]))
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
        alt((cell_is_not_empty, card_already_played, invalid_battle_pick))(i)
    })(i)
}

fn setup_ok(i: &str) -> IResult<&str, SetupOk> {
    response("setup-ok", |i| {
        let (i, battle_system) = prop("battle-system", battle_system)(i)?;
        let (i, blocked_cells) = prop("blocked-cells", blocked_cells)(i)?;
        let (i, hand_blue) = prop("hand-blue", hand)(i)?;
        let (i, hand_red) = prop("hand-red", hand)(i)?;
        let setup_ok = SetupOk {
            battle_system,
            blocked_cells,
            hand_blue,
            hand_red,
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

fn play_ok(i: &str) -> IResult<&str, PlayOk> {
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

    response("play-ok", |i| {
        let (i, events) = opt(prop("events", separated_list0(multispace1, event)))(i)?;
        let (i, pick_battle) = opt(prop(
            "pick-battle",
            map(array(hex_digit_1_n(1)), Into::into),
        ))(i)?;
        let play_ok = PlayOk {
            pick_battle: pick_battle.unwrap_or_default(),
            events: events.unwrap_or_default(),
        };
        Ok((i, play_ok))
    })(i)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{Arrows, BattleSystem, BattleWinner, Battler, Card, Digit, Hand, Player};

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
    const C1P00: Card = Card::physical(1, 0, 0, Arrows(0));
    const C2P00: Card = Card::physical(2, 0, 0, Arrows(0));

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

    #[test_case("()" => BoardCells::NONE)]
    #[test_case("(1)" => BoardCells::from([1]))]
    #[test_case("(2 a B F)" => BoardCells::from([2,0xa,0xb,0xf]))]
    #[test_case("(0 0 0 0 0 0 0)" => panics)]
    #[test_case("(2a)" => panics)]
    #[test_case(" " => panics; "empty string")]
    fn blocked_cells(i: &str) -> BoardCells {
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

    #[test_case("(error CellIsNotEmpty (cell 2))\n" => ErrorResponse::CellIsNotEmpty { cell: 2 })]
    #[test_case("(error CardAlreadyPlayed (card 2))\n" => ErrorResponse::CardAlreadyPlayed { card: 2 })]
    #[test_case("(error InvalidBattlePick (cell 2))\n" => ErrorResponse::InvalidBattlePick { cell: 2 })]
    fn error(i: &str) -> ErrorResponse {
        Response::deserialize(i).unwrap()
    }

    const BLOCKED_CELLS: &str = "(2 3 F)";
    const HAND_BLUE: &str = "(1P00_0 0P00_0 0P00_0 0P00_0 0P00_0)";
    const HAND_RED: &str = "(2P00_0 0P00_0 0P00_0 0P00_0 0P00_0)";
    #[test_case(
        format!("(setup-ok (battle-system dice 9) (blocked-cells {}) (hand-blue {}) (hand-red {}))\n",
            BLOCKED_CELLS, HAND_BLUE, HAND_RED)
        => SetupOk {
            battle_system: BattleSystem::Dice { sides: 9 },
            blocked_cells: [2, 3, 0xf].into(),
            hand_blue: [C1P00, C0P00, C0P00, C0P00, C0P00],
            hand_red: [C2P00, C0P00, C0P00, C0P00, C0P00],
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

    use Event::*;
    #[test_case("(play-ok (events))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![] })]
    #[test_case("(play-ok (events (next-turn blue)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![NextTurn { to: Player::Blue }] })]
    #[test_case("(play-ok (events (next-turn red)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![NextTurn { to: Player::Red }] })]
    #[test_case("(play-ok (events (flip 4)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![Flip { cell: 4 }] })]
    #[test_case("(play-ok (events (flip 2) (flip A) (flip 7)))\n"
        => PlayOk {
            pick_battle: BoardCells::NONE,
            events: vec![Flip { cell: 2 }, Flip { cell: 0xA }, Flip { cell: 7 }] })]
    #[test_case("(play-ok (events (combo-flip A)))\n"
        => PlayOk {
            pick_battle: BoardCells::NONE,
            events: vec![ComboFlip { cell: 0xA }] })]
    #[test_case("(play-ok (events (combo-flip 3) (combo-flip C)))\n"
        => PlayOk {
            pick_battle: BoardCells::NONE,
            events: vec![ComboFlip { cell: 3 }, ComboFlip { cell: 0xC }] })]
    #[test_case("(play-ok (events (battle (1 A D 2) (8 m 3 Cd) attacker)))\n"
        => PlayOk {
            pick_battle: BoardCells::NONE,
            events: vec![Battle {
                attacker: Battler { cell: 1, digit: Digit::Attack, value: 0xD, roll: 2 },
                defender: Battler { cell: 8, digit: Digit::MagicalDefense, value: 3, roll: 0xCD },
                winner: BattleWinner::Attacker,
            }] })]
    #[test_case("(play-ok (events (game-over blue)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![GameOver { winner: Some(Player::Blue) }] })]
    #[test_case("(play-ok (events (game-over red)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![GameOver { winner: Some(Player::Red) }] })]
    #[test_case("(play-ok (events (game-over draw)))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![GameOver { winner: None }] })]
    #[test_case("(play-ok (events) (pick-battle (2 3 4)))\n"
        => PlayOk { pick_battle: [2, 3, 4].into(), events: vec![] })]
    #[test_case("(play-ok (pick-battle (F 4)))\n"
        => PlayOk { pick_battle: [15, 4].into(), events: vec![] })]
    #[test_case("(play-ok (events) (pick-battle ()))\n"
        => PlayOk { pick_battle: BoardCells::NONE, events: vec![] })]
    fn place_card_ok(i: &str) -> PlayOk {
        Response::deserialize(i).unwrap()
    }
}
