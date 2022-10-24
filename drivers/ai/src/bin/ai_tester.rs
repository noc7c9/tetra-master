use clap::Parser;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

use tetra_master_ai as ai;
use tetra_master_core as core;

#[derive(Debug, Parser)]
struct Args {
    /// List all available AIs
    #[arg(long, short)]
    list: bool,

    /// Set seed for global RNG
    #[arg(conflicts_with = "list", long, short)]
    seed: Option<core::Seed>,

    /// Set the amount of time used to test each AI pairing
    #[arg(
        conflicts_with = "list",
        long,
        short,
        name = "SECONDS",
        default_value_t = 10
    )]
    time: u64,

    #[arg(conflicts_with = "list", long, short)]
    continuous: bool,

    #[arg(
        conflicts_with = "list",
        long,
        short,
        value_enum,
        default_value_t = BattleSystemArg::Original
    )]
    battle_system: BattleSystemArg,

    /// Which AIs to test, specify nothing to test all available AIs
    /// At least 2 AIs must by specified
    #[arg(conflicts_with = "list")]
    ais: Vec<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum BattleSystemArg {
    Deterministic,
    Original,
    Dice4,
    Dice6,
    Dice8,
    Dice10,
    Dice12,
}

impl From<BattleSystemArg> for core::BattleSystem {
    fn from(battle_system: BattleSystemArg) -> Self {
        match battle_system {
            BattleSystemArg::Deterministic => core::BattleSystem::Deterministic,
            BattleSystemArg::Original => core::BattleSystem::Original,
            BattleSystemArg::Dice4 => core::BattleSystem::Dice { sides: 4 },
            BattleSystemArg::Dice6 => core::BattleSystem::Dice { sides: 6 },
            BattleSystemArg::Dice8 => core::BattleSystem::Dice { sides: 8 },
            BattleSystemArg::Dice10 => core::BattleSystem::Dice { sides: 10 },
            BattleSystemArg::Dice12 => core::BattleSystem::Dice { sides: 12 },
        }
    }
}

type AiName = &'static str;
type Initializer = Box<dyn Fn(core::Player, &core::Setup) -> Box<dyn ai::Ai>>;

fn main() -> anyhow::Result<()> {
    macro_rules! register {
        (@$all_ais:expr, $name:expr, $mod:ident, $($arg:expr),* $(,)?) => {{
            let initializer: Initializer
                = Box::new(|player, cmd| Box::new(ai::$mod::init($($arg,)* player, cmd)));
            $all_ais.insert($name, initializer);
        }};

        ($all_ais:expr, $mod:ident) => {{
            let name = stringify!($mod);
            register!(@$all_ais, name, $mod,);
        }};
        ($all_ais:expr, $mod:ident as $name:expr) => {{
            register!(@$all_ais, $name, $mod,);
        }};

        ($all_ais:expr, $mod:ident, $($arg:expr),* $(,)?) => {{
            let name = concat!(stringify!($mod), $('_', $arg,)*);
            register!(@$all_ais, name, $mod, $($arg,)*);
        }};
        ($all_ais:expr, $mod:ident as $name:expr, $($arg:expr),* $(,)?) => {{
            register!(@$all_ais, $name, $mod, $($arg,)*);
        }};
    }
    let mut all_ais: HashMap<AiName, Initializer> = HashMap::new();
    // register!(all_ais, random);
    // register!(all_ais, naive_minimax, 3);
    // register!(all_ais, naive_minimax, 4);

    register!(all_ais, expectiminimax_0_naive as "v0", 3);
    register!(all_ais, expectiminimax_1_simplify as "v1", 3);
    register!(all_ais, expectiminimax_2_ab_pruning as "v2", 3);
    register!(all_ais, expectiminimax_3_negamax as "v3", 3);
    register!(all_ais, expectiminimax_4_prob_cutoff as "v4", 3, 0.0);
    register!(all_ais, expectiminimax_5_no_alloc_get_resolutions as "v5", 3, 0.0);
    register!(all_ais, expectiminimax_6_reduce_cloned_data as "v6", 3, 0.0);

    assert!(all_ais.len() >= 2);

    let mut args = Args::parse();
    if args.list {
        list_ais(all_ais.keys().copied());
    } else {
        let ais = if args.ais.is_empty() {
            all_ais.into_iter().collect()
        } else {
            // remove duplicates
            args.ais.sort_unstable();
            args.ais.dedup();

            if args.ais.len() < 2 {
                anyhow::bail!("At least 2 AIs must be specified");
            }

            for name in &args.ais {
                if !all_ais.contains_key(name.as_str()) {
                    anyhow::bail!("The name {name} is not a recognized AI");
                }
            }

            args.ais
                .into_iter()
                .map(|name| all_ais.remove_entry(name.as_str()).unwrap())
                .collect()
        };

        test_ais(
            ais,
            args.battle_system.into(),
            args.seed,
            args.time,
            args.continuous,
        );
    }

    Ok(())
}

fn list_ais(ais: impl Iterator<Item = AiName>) {
    println!("[[Available AIs]]");
    for ai in ais {
        println!("> {}", ai);
    }
}

fn test_ais(
    ais: Vec<(AiName, Initializer)>,
    battle_system: core::BattleSystem,
    global_seed: Option<core::Seed>,
    time_per_pair: u64,
    continuous_mode: bool,
) {
    use crossterm::{
        cursor::{MoveToPreviousLine, RestorePosition, SavePosition},
        execute,
        style::Print,
        terminal::{Clear, ClearType},
    };

    let global_seed = global_seed.unwrap_or_else(|| rand::thread_rng().gen());

    let mut results = Results::new();

    let num_pairings = num_pairings(ais.len());

    if continuous_mode {
        print!("Continuously ");
    }
    print!("Testing {} AIs", ais.len());
    print!(" | total-pairings: {num_pairings}");
    print!(" | time-per-pair: {time_per_pair}s");
    if !continuous_mode {
        let total_expected_time = time_per_pair * num_pairings as u64;
        print!(" | total-expected-time: {total_expected_time}s");
    }
    print!(" | global-seed: {global_seed}");
    print!(" | battle-system: {battle_system:?}");
    print!("\n\n\n");

    let mut stdout = std::io::stdout().lock();

    loop {
        let mut total_expected_time = time_per_pair * num_pairings as u64;
        let mut pairing_count = 0;
        for (i, (ai1_name, ai1)) in ais.iter().enumerate() {
            for (ai2_name, ai2) in &ais[i + 1..] {
                pairing_count += 1;

                let now = std::time::Instant::now();

                let mut elapsed = now.elapsed().as_secs();
                let mut game_count = 0;
                let mut game_seed = 0;

                let mut print_progress = |game_count, game_seed, elapsed| {
                    let time_left = time_per_pair.saturating_sub(elapsed);
                    let total_time_left = total_expected_time.saturating_sub(elapsed);
                    execute!(
                        stdout,
                        MoveToPreviousLine(0),
                        Clear(ClearType::CurrentLine),
                        Print(format!("{ai1_name} v {ai2_name}")),
                        Print(format!(" | pairing: {pairing_count} of {num_pairings}")),
                        Print(format!(" | game {game_count}")),
                        Print(format!(" | time-left: {time_left}s ({total_time_left}s)")),
                        Print(format!(" | seed: {game_seed}")),
                        Print("\n"),
                    )
                    .unwrap()
                };

                let mut global_rng = rand_pcg::Pcg32::seed_from_u64(global_seed);
                while elapsed < time_per_pair {
                    game_seed = global_rng.gen();

                    game_count += 1;
                    print_progress(game_count, game_seed, elapsed);
                    let res = run_battle(battle_system, game_seed, ai1, ai2);
                    results.record(ai1_name, ai2_name, res);

                    elapsed = now.elapsed().as_secs();
                    pause(continuous_mode);

                    game_count += 1;
                    print_progress(game_count, game_seed, elapsed);
                    let res = run_battle(battle_system, game_seed, ai2, ai1);
                    results.record(ai2_name, ai1_name, res);

                    elapsed = now.elapsed().as_secs();
                    pause(continuous_mode);
                }

                print_progress(game_count, game_seed, elapsed);

                total_expected_time -= time_per_pair;
            }
        }

        if continuous_mode {
            execute!(stdout, SavePosition).unwrap();
            println!();
            render_result(results.clone().finalize());
            execute!(stdout, RestorePosition).unwrap();

            pause(continuous_mode);
        } else {
            break;
        }
    }

    println!();
    render_result(results.finalize());
}

fn num_pairings(num_ais: usize) -> usize {
    fn factorial(n: usize) -> usize {
        if n <= 1 {
            1
        } else {
            n * factorial(n - 1)
        }
    }
    factorial(num_ais) / (factorial(num_ais - 2) * 2)
}

#[derive(Debug, Default, Clone)]
struct BattleResult {
    winner: Option<core::Player>,
    blue_ai_move_times: Vec<u128>,
    red_ai_move_times: Vec<u128>,
}

#[derive(Debug, Default, Clone, Copy)]
struct BattleResults {
    wins: usize,
    losses: usize,
    draws: usize,
}

type ResultKey = (AiName, AiName);

#[derive(Debug, Clone)]
struct Results {
    battle_results: HashMap<ResultKey, BattleResults>,
    move_times: HashMap<AiName, Vec<Vec<u128>>>,
}

impl Results {
    fn new() -> Self {
        Self {
            battle_results: HashMap::default(),
            move_times: HashMap::default(),
        }
    }

    fn record(&mut self, blue_ai: AiName, red_ai: AiName, result: BattleResult) {
        match result.winner {
            Some(core::Player::Blue) => self.record_win(blue_ai, red_ai),
            Some(core::Player::Red) => self.record_loss(blue_ai, red_ai),
            None => self.record_draw(blue_ai, red_ai),
        }

        self.move_times
            .entry(blue_ai)
            .or_default()
            .push(result.blue_ai_move_times);
        self.move_times
            .entry(red_ai)
            .or_default()
            .push(result.red_ai_move_times);
    }

    fn record_win(&mut self, ai1: AiName, ai2: AiName) {
        if ai2 < ai1 {
            self.record_loss(ai2, ai1);
            return;
        }
        let key = (ai1, ai2);
        self.battle_results.entry(key).or_default().wins += 1
    }

    fn record_loss(&mut self, ai1: AiName, ai2: AiName) {
        if ai2 < ai1 {
            self.record_win(ai2, ai1);
            return;
        }
        let key = (ai1, ai2);
        self.battle_results.entry(key).or_default().losses += 1
    }

    fn record_draw(&mut self, ai1: AiName, ai2: AiName) {
        let key = if ai2 < ai1 { (ai2, ai1) } else { (ai1, ai2) };
        self.battle_results.entry(key).or_default().draws += 1
    }

    fn finalize(self) -> FinalizedResults {
        let mut ai_names = Vec::new();
        for &(ai1, ai2) in self.battle_results.keys() {
            ai_names.push(ai1);
            ai_names.push(ai2);
        }
        ai_names.sort_unstable();
        ai_names.dedup();

        let mut ai_results: HashMap<AiName, AiResults> = HashMap::with_capacity(ai_names.len());
        for ((ai1, ai2), res) in &self.battle_results {
            let ai1_res = ai_results.entry(ai1).or_default();
            ai1_res.wins += res.wins;
            ai1_res.losses += res.losses;
            ai1_res.draws += res.draws;

            let ai2_res = ai_results.entry(ai2).or_default();
            ai2_res.wins += res.losses;
            ai2_res.losses += res.wins;
            ai2_res.draws += res.draws;
        }

        for (ai, move_times) in self.move_times {
            ai_results.get_mut(ai).unwrap().move_times = move_times;
        }

        FinalizedResults {
            ai_names,
            ai_results,
            battle_results: self.battle_results,
        }
    }
}

#[derive(Debug, Default)]
struct AiResults {
    wins: usize,
    losses: usize,
    draws: usize,
    move_times: Vec<Vec<u128>>,
}

impl AiResults {
    fn total_games(&self) -> usize {
        self.wins + self.losses + self.draws
    }

    fn win_percentage(&self) -> f32 {
        self.wins as f32 / self.total_games() as f32
    }

    fn get_move_times(&self) -> (f64, u128, u128) {
        let mut count = 0;
        let mut total = 0;
        let mut min = u128::MAX;
        let mut max = 0;
        for game in &self.move_times {
            for &datum in game {
                count += 1;
                total += datum;
                if datum < min {
                    min = datum;
                }
                if datum > max {
                    max = datum;
                }
            }
        }
        let avg = total as f64 / count as f64;
        (avg, min, max)
    }
}

#[derive(Debug)]
struct FinalizedResults {
    ai_names: Vec<AiName>,
    ai_results: HashMap<AiName, AiResults>,
    battle_results: HashMap<ResultKey, BattleResults>,
}

impl FinalizedResults {
    fn get_pair(&self, ai1: AiName, ai2: AiName) -> BattleResults {
        if ai2 < ai1 {
            let r = self.get_pair(ai2, ai1);
            return BattleResults {
                wins: r.losses,
                losses: r.wins,
                draws: r.draws,
            };
        }
        let key = (ai1, ai2);
        self.battle_results.get(&key).copied().unwrap_or_default()
    }
}

fn run_battle(
    battle_system: core::BattleSystem,
    game_seed: core::Seed,
    blue_ai: &Initializer,
    red_ai: &Initializer,
) -> BattleResult {
    let mut driver = core::Driver::reference().seed(game_seed).build();
    let setup = driver.random_setup(battle_system);

    let mut ais = [
        blue_ai(core::Player::Blue, &setup),
        red_ai(core::Player::Red, &setup),
    ];

    let mut active_ai = match setup.starting_player {
        core::Player::Blue => 0,
        core::Player::Red => 1,
    };

    driver.send(setup).unwrap();

    let mut move_times = [Vec::with_capacity(7), Vec::with_capacity(7)];

    let mut res: Option<core::PlayOk> = None;
    loop {
        // battle to resolve
        res = if let Some(resolve) = res.and_then(|r| r.resolve_battle) {
            let cmd = driver.resolve_battle(resolve);
            ais[0].apply_resolve_battle(&cmd);
            ais[1].apply_resolve_battle(&cmd);
            Some(driver.send(cmd).unwrap())
        }
        // ai to move
        else {
            let now = std::time::Instant::now();
            let action = ais[active_ai].get_action();
            move_times[active_ai].push(now.elapsed().as_nanos());

            match action {
                ai::Action::PlaceCard(cmd) => {
                    ais[0].apply_place_card(cmd);
                    ais[1].apply_place_card(cmd);
                    Some(driver.send(cmd).unwrap())
                }
                ai::Action::PickBattle(cmd) => {
                    ais[0].apply_pick_battle(cmd);
                    ais[1].apply_pick_battle(cmd);
                    Some(driver.send(cmd).unwrap())
                }
            }
        };

        for event in res.as_ref().unwrap().events.iter() {
            match *event {
                core::Event::NextTurn { .. } => {
                    active_ai = 1 - active_ai;
                }
                core::Event::GameOver { winner } => {
                    let [blue_ai_move_times, red_ai_move_times] = move_times;
                    return BattleResult {
                        winner,
                        blue_ai_move_times,
                        red_ai_move_times,
                    };
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Alignment {
    Left,
    Right,
    Center,
}

struct Table {
    rows: usize,
    cols: usize,
    cells: Vec<String>,
    alignment: Vec<Alignment>,
}

impl Table {
    fn new(rows: usize, cols: usize) -> Self {
        let size = rows * cols;
        Self {
            rows,
            cols,
            cells: vec![String::new(); size], // shouldn't allocate any strings
            alignment: vec![Alignment::Left; size],
        }
    }

    fn idx(&self, row: usize, col: usize) -> usize {
        debug_assert!(row < self.rows);
        debug_assert!(col < self.cols);
        row * self.cols + col
    }

    fn set(&mut self, row: usize, col: usize, value: String) {
        let idx = self.idx(row, col);
        self.cells[idx] = value;
    }

    fn set_left(&mut self, row: usize, col: usize, value: String) {
        let idx = self.idx(row, col);
        self.cells[idx] = value;
        self.alignment[idx] = Alignment::Left;
    }

    fn set_right(&mut self, row: usize, col: usize, value: String) {
        let idx = self.idx(row, col);
        self.cells[idx] = value;
        self.alignment[idx] = Alignment::Right;
    }

    fn set_center(&mut self, row: usize, col: usize, value: String) {
        let idx = self.idx(row, col);
        self.cells[idx] = value;
        self.alignment[idx] = Alignment::Center;
    }

    fn get(&self, row: usize, col: usize) -> &str {
        let idx = self.idx(row, col);
        &self.cells[idx]
    }

    fn get_alignment(&self, row: usize, col: usize) -> Alignment {
        let idx = self.idx(row, col);
        self.alignment[idx]
    }

    fn render(self) {
        let col_widths = (0..self.cols)
            .map(|col| (0..self.rows).map(|row| self.get(row, col).len()).max())
            .collect::<Option<Vec<_>>>()
            .unwrap();

        let print_sep = |start, mid, end| {
            print!("{}", start);
            let mut col_widths = col_widths.iter().copied();
            print!("━{}━", "━".repeat(col_widths.next().unwrap()));
            for col_width in col_widths {
                print!("{mid}━{}━", "━".repeat(col_width));
            }
            println!("{}", end);
        };
        let print_row = |row| {
            print!("┃");
            for (i, col) in (0..self.cols).enumerate() {
                let value = self.get(row, col);
                match self.get_alignment(row, col) {
                    Alignment::Left => print!(" {0:<1$} ┃", value, col_widths[i]),
                    Alignment::Right => print!(" {0:>1$} ┃", value, col_widths[i]),
                    Alignment::Center => print!(" {0:^1$} ┃", value, col_widths[i]),
                }
            }
            println!();
        };

        print_sep("┏", "┳", "┓");

        let mut rows = 0..self.rows;
        print_row(rows.next().unwrap());
        print_sep("┣", "╋", "┫");
        for row in rows {
            print_row(row);
        }

        print_sep("┗", "┻", "┛");
    }
}

fn render_result(results: FinalizedResults) {
    // render table of battle results for each AI v AI pairing
    let mut table = Table::new(results.ai_names.len() + 1, results.ai_names.len() + 1);
    table.set_center(0, 0, "W / L / D".into());
    for (i, &name) in results.ai_names.iter().enumerate() {
        table.set_left(0, 1 + i, name.into()); // header
        table.set_right(1 + i, 0, name.into()); // side
    }
    for (i, ai1) in results.ai_names.iter().enumerate() {
        for (j, ai2) in results.ai_names.iter().enumerate() {
            let value = if ai1 == ai2 {
                "───".into()
            } else {
                let res = results.get_pair(ai1, ai2);
                format!("{} / {} / {}", res.wins, res.losses, res.draws)
            };
            table.set_center(1 + i, 1 + j, value);
        }
    }
    table.render();

    println!();

    // render tables of each AIs results in order of strength
    let mut sorted: Vec<_> = results.ai_results.into_iter().collect();
    sorted.sort_by_key(|(_, res)| TotalOrd(-res.win_percentage()));

    // total wins, losses, draws, games
    let mut table = Table::new(results.ai_names.len() + 1, 5);
    table.set(0, 1, "Wins".into());
    table.set(0, 2, "Losses".into());
    table.set(0, 3, "Draws".into());
    table.set(0, 4, "Games".into());
    for (row, (name, res)) in sorted.iter().enumerate() {
        table.set_right(row + 1, 0, name.to_string());

        let total = res.total_games() as f32;
        for (col, value) in [res.wins, res.losses, res.draws].into_iter().enumerate() {
            let value = format!("{} ({:.2}%)", value, value as f32 / total * 100.);
            table.set(row + 1, col + 1, value);
        }
        table.set(row + 1, 4, format!("{total}"));
    }
    table.render();

    println!();

    // render move times table of each AIs results in order of move time
    sorted.sort_by_cached_key(|(_, res)| TotalOrd(res.get_move_times().0));

    // move times
    let mut table = Table::new(results.ai_names.len() + 1, 4);
    table.set(0, 1, "Avg. Move Time".into());
    table.set(0, 2, "Min. Move Time".into());
    table.set(0, 3, "Max. Move Time".into());
    for (row, (name, res)) in sorted.iter().enumerate() {
        table.set_right(row + 1, 0, name.to_string());

        let (avg, min, max) = res.get_move_times();
        table.set(row + 1, 1, format_time(avg as u128));
        table.set(row + 1, 2, format_time(min));
        table.set(row + 1, 3, format_time(max));
    }
    table.render();
}

fn format_time(nanos: u128) -> String {
    // seconds
    if nanos > 1_000_000_000 {
        let secs = nanos as f64 / 1_000_000_000.0;
        format!("{secs:.2} s ({nanos} ns)")
    }
    // milliseconds
    else if nanos > 1_000_000 {
        let millis = nanos as f64 / 1_000_000.0;
        format!("{millis:.2} ms ({nanos} ns)")
    }
    // microseconds
    else if nanos > 1_000 {
        let micros = nanos as f64 / 1_000.0;
        format!("{micros:.2} us ({nanos} ns)")
    }
    // nanoseconds
    else {
        format!("{nanos} ns")
    }
}

#[derive(Debug)]
struct TotalOrd<T>(T);

macro_rules! impl_total_ord {
    ($ty:ty) => {
        impl PartialEq for TotalOrd<$ty> {
            fn eq(&self, other: &Self) -> bool {
                self.0.total_cmp(&other.0).is_eq()
            }
        }
        impl Eq for TotalOrd<$ty> {}
        impl PartialOrd for TotalOrd<$ty> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.0.total_cmp(&other.0))
            }
        }
        impl Ord for TotalOrd<$ty> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.total_cmp(&other.0)
            }
        }
    };
}
impl_total_ord!(f32);
impl_total_ord!(f64);

// to not cause 100% cpu usage
fn pause(on: bool) {
    if on {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
