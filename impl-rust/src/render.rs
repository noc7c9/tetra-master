use crate::{
    BattleWinner, Card, CardType, Cell, Entry, GameLog, GameState, GameStatus, OwnedCard, Player,
};
use std::fmt::Write;

const RED: &str = "\x1b[0;31m";
const RED_BOLD: &str = "\x1b[1;31m";
const BLUE: &str = "\x1b[0;34m";
const BLUE_BOLD: &str = "\x1b[1;34m";
const GRAY: &str = "\x1b[0;30m";
const GRAY_BOLD: &str = "\x1b[1;30m";
const WHITE_BOLD: &str = "\x1b[1;37m";
const RESET: &str = "\x1b[0m";

type Result = std::result::Result<(), std::fmt::Error>;

pub(crate) fn screen(log: &GameLog, state: &GameState, o: &mut String) -> Result {
    write!(o, "\x1b]50;ClearScrollback\x07")?;

    push_hand(o, Player::P1, &state.p1_hand)?;
    writeln!(o)?;

    push_board(o, state)?;
    writeln!(o)?;

    push_hand(o, Player::P2, &state.p2_hand)?;
    writeln!(o)?;

    push_game_log(o, log)?;

    if let GameStatus::GameOver { winner } = state.status {
        push_game_over(o, winner)
    } else {
        push_prompt(o, state)
    }
}

fn push_hand(o: &mut String, owner: Player, hand: &[Option<Card>; 5]) -> Result {
    write!(o, "{}", owner.to_color())?;

    // line 1
    for (idx, card) in hand.iter().enumerate() {
        if card.is_some() {
            write!(o, "╔═══ {idx:X} ═══╗")?;
        } else {
            write!(o, "           ")?;
        }
    }
    writeln!(o)?;

    // line 2
    for card in hand {
        if let Some(card) = card {
            let ul = if card.arrows.up_left() { '⇖' } else { ' ' };
            let u = if card.arrows.up() { '⇑' } else { ' ' };
            let ur = if card.arrows.up_right() { '⇗' } else { ' ' };
            write!(o, "║ {ul}  {u}  {ur} ║")?;
        } else {
            write!(o, "           ")?;
        }
    }
    writeln!(o)?;

    // line 3
    for card in hand {
        if let Some(card) = card {
            let l = if card.arrows.left() { '⇐' } else { ' ' };
            let r = if card.arrows.right() { '⇒' } else { ' ' };
            let stats = Stats::from(card);
            write!(o, "║ {l} {stats}{r} ║")?;
        } else {
            write!(o, "           ")?;
        }
    }
    writeln!(o)?;

    // line 4
    for card in hand {
        if let Some(card) = card {
            let dl = if card.arrows.down_left() { '⇙' } else { ' ' };
            let d = if card.arrows.down() { '⇓' } else { ' ' };
            let dr = if card.arrows.down_right() { '⇘' } else { ' ' };
            write!(o, "║ {dl}  {d}  {dr} ║")?;
        } else {
            write!(o, "           ")?;
        }
    }
    writeln!(o)?;

    // line 5
    for card in hand {
        if card.is_some() {
            write!(o, "╚═════════╝")?;
        } else {
            write!(o, "           ")?;
        }
    }
    writeln!(o, "{RESET}")
}

fn push_board(o: &mut String, state: &GameState) -> Result {
    writeln!(o, "   ┌───────────┬───────────┬───────────┬───────────┐")?;

    for (idx, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        // line 1 in row
        write!(o, "   │")?;
        for j in row {
            match &state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    let ul = if card.arrows.up_left() { '⇖' } else { ' ' };
                    let u = if card.arrows.up() { '⇑' } else { ' ' };
                    let ur = if card.arrows.up_right() { '⇗' } else { ' ' };
                    write!(o, "{} {ul}   {u}   {ur} {RESET}", owner.to_color())?;
                }
                Cell::Blocked => {
                    write!(o, "{GRAY_BOLD} ╔═══════╗ {RESET}")?;
                }
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "│")?;
        }

        // line 2 in row
        write!(o, "\n   │")?;
        for j in row {
            if let Cell::Blocked = &state.board[j] {
                write!(o, "{GRAY_BOLD} ║       ║ {RESET}")?;
            } else {
                write!(o, "           ")?;
            }
            write!(o, "│")?;
        }
        write!(o, "\n   │")?;

        // line 3 in row
        for j in row {
            match state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    let l = if card.arrows.left() { '⇐' } else { ' ' };
                    let r = if card.arrows.right() { '⇒' } else { ' ' };
                    let stats = Stats::from(card);
                    write!(o, "{} {l}  {stats} {r} {RESET}", owner.to_color())?;
                }
                Cell::Blocked => {
                    write!(o, "{GRAY_BOLD} ║ BLOCK ║ {RESET}")?;
                }
                Cell::Empty => {
                    write!(o, "     {j:X}     ")?;
                }
            }
            write!(o, "│")?;
        }

        // line 4 in row
        write!(o, "\n   │")?;
        for j in row {
            if let Cell::Blocked = &state.board[j] {
                write!(o, "{GRAY_BOLD} ║       ║ {RESET}")?;
            } else {
                write!(o, "           ")?;
            }
            write!(o, "│")?;
        }
        write!(o, "\n   │")?;

        // line 5 in row
        for j in row {
            match &state.board[j] {
                Cell::Card(OwnedCard { owner, card }) => {
                    let dl = if card.arrows.down_left() { '⇙' } else { ' ' };
                    let d = if card.arrows.down() { '⇓' } else { ' ' };
                    let dr = if card.arrows.down_right() { '⇘' } else { ' ' };
                    write!(o, "{} {dl}   {d}   {dr} {RESET}", owner.to_color())?;
                }
                Cell::Blocked => {
                    write!(o, "{GRAY_BOLD} ╚═══════╝ {RESET}")?;
                }
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "│")?;
        }

        if idx != 3 {
            writeln!(o, "\n   ├───────────┼───────────┼───────────┼───────────┤")?;
        }
    }

    writeln!(o, "\n   └───────────┴───────────┴───────────┴───────────┘")
}

fn push_game_log(o: &mut String, log: &GameLog) -> Result {
    writeln!(o, "                   {GRAY_BOLD} ══ GAMELOG ══ {RESET}")?;

    let mut curr_turn_number = 1;
    let mut curr_turn = Player::P1;
    let mut print_prefix = true;
    for entry in log.iter() {
        if let Entry::NextTurn { turn } = entry {
            curr_turn_number += 1;
            curr_turn = *turn;
            print_prefix = true;
            continue;
        }

        o.push_str(curr_turn.to_color());
        if !print_prefix {
            write!(o, "           ")?;
        } else if curr_turn_number < 10 {
            write!(o, "    Turn {curr_turn_number} ")?;
        } else {
            write!(o, "   Turn 10 ")?;
        }
        print_prefix = false;
        write!(o, "{RESET}│ ")?;

        match entry {
            Entry::PlaceCard { card, cell } => {
                let stats = Stats::from(card);
                write!(o, "Placed  {stats} on cell {cell:X}")?;
            }

            Entry::FlipCard {
                card,
                cell,
                to,
                via_combo,
            } => {
                let prefix = if *via_combo { "Combo'd " } else { "Flipped " };
                let stats = Stats::from(card);
                write!(o, "{prefix}{stats} on cell {cell:X} to {to}")?;
            }

            Entry::Battle {
                attacker,
                defender,
                result,
            } => {
                let att_stats = Stats::from(attacker).highlight(result.attack_stat.digit);
                let def_stats = Stats::from(defender).highlight(result.defense_stat.digit);
                writeln!(o, "Battle  {att_stats} vs {def_stats}")?;

                let att_color = attacker.owner.to_color();
                let att_value = result.attack_stat.value;
                let att_roll = result.attack_stat.roll;
                let att_resolve = result.attack_stat.resolve();

                let def_color = defender.owner.to_color();
                let def_value = result.defense_stat.value;
                let def_roll = result.defense_stat.roll;
                let def_resolve = result.defense_stat.resolve();

                write!(o, "           {RESET}│         ")?;

                write!(o, "{}Attacker{RESET} ", att_color)?;
                write!(o, "({}) rolled {}, ", att_value, att_roll)?;

                write!(o, "{}Defender{RESET} ", def_color)?;
                writeln!(o, "({}) rolled {}", def_value, def_roll)?;

                match result.winner {
                    BattleWinner::Attacker => {
                        write!(o, "           │         {}Attacker wins{RESET} ", att_color)?;
                        write!(o, "({} > {})", att_resolve, def_resolve)?;
                    }
                    BattleWinner::Defender => {
                        write!(o, "           │         {}Defender wins{RESET} ", def_color)?;
                        write!(o, "({} < {})", att_resolve, def_resolve)?;
                    }
                    BattleWinner::None => {
                        write!(o, "           │         Draw, ")?;
                        write!(o, "{}defender wins{RESET} ", def_color)?;
                        write!(o, "by default ({} = {})", att_resolve, def_resolve)?;
                    }
                }
            }

            Entry::NextTurn { .. } => unreachable!(),
        }
        writeln!(o)?;
    }

    Ok(())
}

fn push_prompt(o: &mut String, state: &GameState) -> Result {
    let prefix = match state.turn {
        Player::P1 => "",
        Player::P2 => " ",
    };
    write!(o, "{prefix}Next: {} │ ", state.turn)?;

    match &state.status {
        GameStatus::WaitingPlace => {
            write!(o, "Where to place which card? ")?;
            writeln!(o, "{GRAY}( format: {{CARD#}} {{COORD}} ){RESET}")?;
        }
        GameStatus::WaitingBattle { choices, .. } => {
            write!(o, "Which card to battle? ")?;
            writeln!(o, "{GRAY}( format: {{COORD}} ){RESET}")?;
            for &(cell, card) in choices {
                let stats = Stats::from(card).owner(state.turn.opposite());
                writeln!(o, "  {cell:X} ( {stats} )")?;
            }
        }
        GameStatus::GameOver { .. } => unreachable!(),
    }

    Ok(())
}

fn push_game_over(o: &mut String, winner: Option<Player>) -> Result {
    write!(o, " {WHITE_BOLD}Game Over{RESET} │ ")?;
    match winner {
        Some(winner) => {
            writeln!(o, "{winner} Wins")?;
        }
        None => {
            writeln!(o, "It was a draw!")?;
        }
    }

    Ok(())
}

impl Player {
    fn to_color(self) -> &'static str {
        match self {
            Player::P1 => BLUE,
            Player::P2 => RED,
        }
    }

    fn to_color_bold(self) -> &'static str {
        match self {
            Player::P1 => BLUE_BOLD,
            Player::P2 => RED_BOLD,
        }
    }
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Player::P1 => "Blue",
            Player::P2 => "Red",
        };
        write!(f, "{}{name}{RESET}", self.to_color())
    }
}

struct Stats {
    card: Card,
    owner: Option<Player>,
    highlight: Option<u8>,
}

impl Stats {
    fn owner(mut self, owner: Player) -> Self {
        self.owner = Some(owner);
        self
    }

    fn highlight(mut self, highlight: u8) -> Self {
        self.highlight = Some(highlight);
        self
    }
}

impl From<Card> for Stats {
    fn from(card: Card) -> Self {
        Stats {
            card,
            owner: None,
            highlight: None,
        }
    }
}

impl From<OwnedCard> for Stats {
    fn from(owned: OwnedCard) -> Self {
        Stats {
            card: owned.card,
            owner: Some(owned.owner),
            highlight: None,
        }
    }
}

impl From<&Card> for Stats {
    fn from(card: &Card) -> Self {
        (*card).into()
    }
}

impl From<&OwnedCard> for Stats {
    fn from(owned: &OwnedCard) -> Self {
        (*owned).into()
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let att = self.card.attack >> 4;
        let phy = self.card.physical_defense >> 4;
        let mag = self.card.magical_defense >> 4;
        let typ = match self.card.card_type {
            CardType::Physical => 'P',
            CardType::Magical => 'M',
            CardType::Exploit => 'X',
            CardType::Assault => 'A',
        };

        if let Some(owner) = self.owner {
            let color = owner.to_color();
            let color_bold = owner.to_color_bold();
            let highlight = self.highlight.unwrap_or(u8::MAX);

            let c = if highlight == 0 { color_bold } else { color };
            write!(f, "{c}{att:X}")?;

            write!(f, "{color}{typ}")?;

            let c = if highlight == 2 { color_bold } else { color };
            write!(f, "{c}{phy:X}")?;

            let c = if highlight == 3 { color_bold } else { color };
            write!(f, "{c}{mag:X}{RESET}")
        } else {
            write!(f, "{att:X}{typ}{phy:X}{mag:X}")
        }
    }
}
