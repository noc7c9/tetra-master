use crate::{
    BattleSystem, BattleWinner, Board, Card, CardType, Cell, Entry, GameLog, GameState, GameStatus,
    OwnedCard, Player, PreGameState, PreGameStatus,
};
use std::fmt::Write;

// src: figlet font, Alligator2 by Daniel Wiz. AKA Merlin Greywolf <merlin@brahms.udel.edu>
const NUMBERS: [[&str; 7]; 11] = [
    [
        "    :::::::",
        "   :+:   :+:",
        "   +:+  :+:+",
        "   +#+ + +:+",
        "   +#+#  +#+",
        "   #+#   #+#",
        "    #######",
    ],
    [
        "     :::",
        "   :+:+:",
        "     +:+",
        "     +#+",
        "     +#+",
        "     #+#",
        "   #######",
    ],
    [
        "    ::::::::",
        "   :+:    :+:",
        "         +:+",
        "       +#+",
        "     +#+",
        "    #+#",
        "   ##########",
    ],
    [
        "    ::::::::",
        "   :+:    :+:",
        "          +:+",
        "       +#++:",
        "          +#+",
        "   #+#    #+#",
        "    ########",
    ],
    [
        "       :::",
        "      :+:",
        "     +:+ +:+",
        "    +#+  +:+",
        "   +#+#+#+#+#+",
        "         #+#",
        "         ###",
    ],
    [
        "   ::::::::::",
        "   :+:    :+:",
        "   +:+",
        "   +#++:++#+",
        "          +#+",
        "   #+#    #+#",
        "    ########",
    ],
    [
        "    ::::::::",
        "   :+:    :+:",
        "   +:+",
        "   +#++:++#+",
        "   +#+    +#+",
        "   #+#    #+#",
        "    ########",
    ],
    [
        "   :::::::::::",
        "   :+:     :+:",
        "          +:+",
        "         +#+",
        "        +#+",
        "       #+#",
        "       ###",
    ],
    [
        "    ::::::::",
        "   :+:    :+:",
        "   +:+    +:+",
        "    +#++:++#",
        "   +#+    +#+",
        "   #+#    #+#",
        "    ########",
    ],
    [
        "    ::::::::",
        "   :+:    :+:",
        "   +:+    +:+",
        "    +#++:++#+",
        "          +#+",
        "   #+#    #+#",
        "    ########",
    ],
    [
        "     :::  :::::::",
        "   :+:+: :+:   :+:",
        "     +:+ +:+  :+:+",
        "     +#+ +#+ + +:+",
        "     +#+ +#+#  +#+",
        "     #+# #+#   #+#",
        "   ##############",
    ],
];

type Result = std::result::Result<(), std::fmt::Error>;

pub(crate) fn pre_game_screen(o: &mut String, state: &PreGameState) -> Result {
    clear_screen(o)?;

    match state.status {
        PreGameStatus::P1Picking => {
            push_board(o, &state.board)?;

            for (idx, hand) in state.hand_candidates.iter().enumerate() {
                writeln!(o, "Hand {idx}")?;
                push_hand_candidate(o, hand)?;
            }

            writeln!(o, "Player 1 pick a hand (Player 2 will pick next)")?;
        }
        PreGameStatus::P2Picking { p1_pick } => {
            writeln!(o, "Player 1")?;
            push_hand_candidate(o, &state.hand_candidates[p1_pick])?;

            push_board(o, &state.board)?;

            for (idx, hand) in state.hand_candidates.iter().enumerate() {
                if idx != p1_pick {
                    writeln!(o, "Hand {idx}")?;
                    push_hand_candidate(o, hand)?;
                }
            }

            writeln!(o, "Player 2 pick a hand?")?;
        }
        PreGameStatus::Complete { p1_pick, p2_pick } => {
            writeln!(o, "Player 1")?;
            push_hand_candidate(o, &state.hand_candidates[p1_pick])?;

            push_board(o, &state.board)?;

            writeln!(o, "Player 2")?;
            push_hand_candidate(o, &state.hand_candidates[p2_pick])?;
        }
    }

    Ok(())
}

pub(crate) fn game_screen(o: &mut String, log: &GameLog, state: &GameState) -> Result {
    clear_screen(o)?;

    if state.turn == Player::P1 {
        write!(o, ">> ")?;
    }
    writeln!(o, "Player 1")?;
    push_hand(o, &state.p1_hand)?;

    push_board(o, &state.board)?;

    if state.turn == Player::P2 {
        write!(o, ">> ")?;
    }
    writeln!(o, "Player 2")?;
    push_hand(o, &state.p2_hand)?;

    push_game_log(o, log, &state.battle_system)?;

    if let GameStatus::GameOver { winner } = state.status {
        push_game_over(o, winner)
    } else {
        push_prompt(o, state)
    }
}

fn clear_screen(o: &mut String) -> Result {
    // print multiple new lines to "clear the screen"
    for _ in 0..100 {
        writeln!(o)?;
    }

    Ok(())
}

fn push_hand_candidate(o: &mut String, &[a, b, c, d, e]: &[Card; 5]) -> Result {
    push_hand(o, &[Some(a), Some(b), Some(c), Some(d), Some(e)])
}

fn push_hand(o: &mut String, hand: &[Option<Card>; 5]) -> Result {
    // line 1
    for (idx, card) in hand.iter().enumerate() {
        if card.is_some() {
            write!(o, "+--- {idx:X} ---+ ")?;
        } else {
            write!(o, "            ")?;
        }
    }
    writeln!(o)?;

    // line 2
    for card in hand {
        if let Some(card) = card {
            let ul = if card.arrows.up_left() { '\\' } else { ' ' };
            let u = if card.arrows.up() { '|' } else { ' ' };
            let ur = if card.arrows.up_right() { '/' } else { ' ' };
            write!(o, "| {ul}  {u}  {ur} | ")?;
        } else {
            write!(o, "            ")?;
        }
    }
    writeln!(o)?;

    // line 3
    for card in hand {
        if let Some(card) = card {
            let l = if card.arrows.left() { '-' } else { ' ' };
            let r = if card.arrows.right() { '-' } else { ' ' };
            let stats = Stats::from(card);
            write!(o, "| {l} {stats}{r} | ")?;
        } else {
            write!(o, "            ")?;
        }
    }
    writeln!(o)?;

    // line 4
    for card in hand {
        if let Some(card) = card {
            let dl = if card.arrows.down_left() { '/' } else { ' ' };
            let d = if card.arrows.down() { '|' } else { ' ' };
            let dr = if card.arrows.down_right() { '\\' } else { ' ' };
            write!(o, "| {dl}  {d}  {dr} | ")?;
        } else {
            write!(o, "            ")?;
        }
    }
    writeln!(o)?;

    // line 5
    for card in hand {
        if card.is_some() {
            write!(o, "+---------+ ")?;
        } else {
            write!(o, "            ")?;
        }
    }
    writeln!(o, "\n")
}

fn push_board(o: &mut String, board: &Board) -> Result {
    writeln!(o, "   +-----------+-----------+-----------+-----------+")?;

    let (p1_card_count, p2_card_count) = board.iter().fold((0, 0), |(p1, p2), cell| match cell {
        Cell::Card(OwnedCard { owner, .. }) if *owner == Player::P1 => (p1 + 1, p2),
        Cell::Card(OwnedCard { owner, .. }) if *owner == Player::P2 => (p1, p2 + 1),
        _ => (p1, p2),
    });

    for (idx, &row) in [[0, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [12, 13, 14, 15]]
        .iter()
        .enumerate()
    {
        // line 1 in row
        write!(o, "   |")?;
        for j in row {
            match &board[j] {
                Cell::Card(OwnedCard { owner, .. }) => {
                    write!(o, " {}        ", DisplayPlayer(*owner))?
                }
                Cell::Blocked => write!(o, " ######### ")?,
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "|")?;
        }
        if idx == 1 {
            write!(o, "{}", NUMBERS[p1_card_count][4])?;
        }
        if idx == 3 {
            write!(o, "{}", NUMBERS[p2_card_count][4])?;
        }

        // line 2 in row
        write!(o, "\n   |")?;
        for j in row {
            match &board[j] {
                Cell::Card(OwnedCard { card, .. }) => {
                    let ul = if card.arrows.up_left() { '\\' } else { ' ' };
                    let u = if card.arrows.up() { '|' } else { ' ' };
                    let ur = if card.arrows.up_right() { '/' } else { ' ' };
                    write!(o, "  {ul}  {u}  {ur}  ")?;
                }
                Cell::Blocked => write!(o, " #       # ")?,
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "|")?;
        }
        if idx == 1 {
            write!(o, "{}", NUMBERS[p1_card_count][5])?;
        }
        if idx == 3 {
            write!(o, "{}", NUMBERS[p2_card_count][5])?;
        }

        // line 3 in row
        write!(o, "\n   |")?;
        for j in row {
            match board[j] {
                Cell::Card(OwnedCard { card, .. }) => {
                    let l = if card.arrows.left() { '-' } else { ' ' };
                    let r = if card.arrows.right() { '-' } else { ' ' };
                    let stats = Stats::from(card);
                    write!(o, "  {l} {stats}{r}  ")?;
                }
                Cell::Blocked => write!(o, " # BLOCK # ")?,
                Cell::Empty => write!(o, "     {j:X}     ")?,
            }
            write!(o, "|")?;
        }
        if idx == 0 {
            write!(o, "{}", NUMBERS[p1_card_count][0])?;
        }
        if idx == 1 {
            write!(o, "{}", NUMBERS[p1_card_count][6])?;
        }
        if idx == 2 {
            write!(o, "{}", NUMBERS[p2_card_count][0])?;
        }
        if idx == 3 {
            write!(o, "{}", NUMBERS[p2_card_count][6])?;
        }

        // line 4 in row
        write!(o, "\n   |")?;
        for j in row {
            match &board[j] {
                Cell::Card(OwnedCard { card, .. }) => {
                    let dl = if card.arrows.down_left() { '/' } else { ' ' };
                    let d = if card.arrows.down() { '|' } else { ' ' };
                    let dr = if card.arrows.down_right() { '\\' } else { ' ' };
                    write!(o, "  {dl}  {d}  {dr}  ")?;
                }
                Cell::Blocked => write!(o, " #       # ")?,
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "|")?;
        }
        if idx == 0 {
            write!(o, "{}", NUMBERS[p1_card_count][1])?;
        }
        if idx == 2 {
            write!(o, "{}", NUMBERS[p2_card_count][1])?;
        }

        // line 5 in row
        write!(o, "\n   |")?;
        for j in row {
            match &board[j] {
                Cell::Card(OwnedCard { owner, .. }) => {
                    write!(o, "        {} ", DisplayPlayer(*owner))?
                }
                Cell::Blocked => write!(o, " ######### ")?,
                Cell::Empty => write!(o, "           ")?,
            }
            write!(o, "|")?;
        }
        if idx == 0 {
            write!(o, "{}", NUMBERS[p1_card_count][2])?;
        }
        if idx == 2 {
            write!(o, "{}", NUMBERS[p2_card_count][2])?;
        }

        if idx != 3 {
            write!(o, "\n   +-----------+-----------+-----------+-----------+")?;
            if idx == 0 {
                write!(o, "{}", NUMBERS[p1_card_count][3])?;
            }
            if idx == 2 {
                write!(o, "{}", NUMBERS[p2_card_count][3])?;
            }
            writeln!(o)?;
        }
    }

    writeln!(o, "\n   +-----------+-----------+-----------+-----------+")?;
    writeln!(o)
}

fn push_game_log(o: &mut String, log: &GameLog, battle_system: &BattleSystem) -> Result {
    writeln!(o, "                    ══ GAMELOG ══ ")?;

    let mut curr_turn_number = 0;
    let mut print_prefix = true;
    for entry in log.iter() {
        if let Entry::NextTurn { .. } = entry {
            curr_turn_number += 1;
            print_prefix = true;
            continue;
        }

        if !print_prefix {
            write!(o, "           ")?;
        } else if curr_turn_number < 10 {
            write!(o, "    Turn {curr_turn_number} ")?;
        } else {
            write!(o, "   Turn 10 ")?;
        }
        print_prefix = false;
        write!(o, "| ")?;

        match entry {
            Entry::PreGameSetup {
                seed,
                p1_pick,
                p2_pick,
            } => {
                match battle_system {
                    BattleSystem::Original => writeln!(o, "Using the Original battle system")?,
                    BattleSystem::Dice { sides } => {
                        writeln!(o, "Using the Dice battle system with {sides} sided die")?
                    }
                    BattleSystem::External { rolls, .. } => {
                        writeln!(o, "Using the External battle system (rolls: {rolls:?})")?
                    }
                }
                writeln!(o, "           | The RNG seed is {seed}")?;
                write!(o, "           | Player 1 picked hand {p1_pick}, ")?;
                write!(o, "Player 2 picked hand {p2_pick}")?;
            }

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
                let to = DisplayPlayer(*to);
                write!(o, "{prefix}{stats} on cell {cell:X} to {to}")?;
            }

            Entry::Battle {
                attacker,
                defender,
                result,
                ..
            } => {
                let att_stats = Stats::from(attacker).highlight(result.attack_stat.digit);
                let def_stats = Stats::from(defender).highlight(result.defense_stat.digit);
                writeln!(o, "Battle  {att_stats} vs {def_stats}")?;

                let att_value = result.attack_stat.value;
                let att_roll = result.attack_stat.roll;

                let def_value = result.defense_stat.value;
                let def_roll = result.defense_stat.roll;

                write!(o, "           |         ")?;

                match battle_system {
                    BattleSystem::Original | BattleSystem::External { .. } => {
                        write!(o, "Attacker ({att_value}) rolled {att_roll}, ")?;
                        writeln!(o, "Defender ({def_value}) rolled {def_roll}")?;
                    }
                    BattleSystem::Dice { sides } => {
                        write!(o, "Attacker ({att_value}d{sides}) rolled {att_roll}, ")?;
                        writeln!(o, "Defender ({def_value}d{sides}) rolled {def_roll}")?;
                    }
                }

                match result.winner {
                    BattleWinner::Attacker => {
                        write!(o, "           |         Attacker wins ")?;
                        write!(o, "({att_roll} > {def_roll})")?;
                    }
                    BattleWinner::Defender => {
                        write!(o, "           |         Defender wins ")?;
                        write!(o, "({att_roll} < {def_roll})")?;
                    }
                    BattleWinner::None => {
                        write!(o, "           |         Draw, ")?;
                        write!(o, "defender wins ")?;
                        write!(o, "by default ({att_roll} = {def_roll})")?;
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
    writeln!(o, "           | ")?;
    write!(o, "  Next: {} | ", DisplayPlayer(state.turn))?;

    match &state.status {
        GameStatus::WaitingPlace => {
            write!(o, "Where to place which card? ")?;
            writeln!(o, "( format: {{CARD#}} {{COORD}} )")?;
        }
        GameStatus::WaitingBattle { choices, .. } => {
            writeln!(o, "Which card to battle? ( format: {{COORD}} )")?;
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
    write!(o, " Game Over | ")?;
    match winner {
        Some(winner) => {
            writeln!(o, "{} Wins", DisplayPlayer(winner))?;
        }
        None => {
            writeln!(o, "It was a draw!")?;
        }
    }

    Ok(())
}

struct DisplayPlayer(Player);

impl std::fmt::Display for DisplayPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            Player::P1 => "P1",
            Player::P2 => "P2",
        };
        write!(f, "{name}")
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
        let att = self.card.attack;
        let phy = self.card.physical_defense;
        let mag = self.card.magical_defense;
        let typ = match self.card.card_type {
            CardType::Physical => 'P',
            CardType::Magical => 'M',
            CardType::Exploit => 'X',
            CardType::Assault => 'A',
        };

        let highlight = self.highlight.unwrap_or(u8::MAX);

        if highlight == 0 {
            write!(f, "[{att:X}]")?;
        } else {
            write!(f, "{att:X}")?;
        }

        write!(f, "{typ}")?;

        if highlight == 2 {
            write!(f, "[{phy:X}]")?;
        } else {
            write!(f, "{phy:X}")?;
        };

        if highlight == 3 {
            write!(f, "[{mag:X}]")
        } else {
            write!(f, "{mag:X}")
        }
    }
}
