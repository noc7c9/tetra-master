use tetra_master_ai as ai;
use tetra_master_core as core;

// Hard-code seeds for deterministic results
const GAME_SEED: u64 = 11237964071758638171;
// const GAME_SEED: u64 = 14604420560562181961;
// const GAME_SEED: u64 = 5404026930181620362;
// const GAME_SEED: u64 = 11239310865536683934;
// const GAME_SEED: u64 = 11613753253682439726;
// const GAME_SEED: u64 = 17461004733143158712;
// const GAME_SEED: u64 = 5382151945609755983;
// const GAME_SEED: u64 = 12223005711769424545;

macro_rules! benchmark {
    ($mod:ident, $($arg:expr),* $(,)?) => {
        fn $mod() {
            use ai::Ai;

            let mut driver = core::Driver::reference().seed(GAME_SEED).build();
            let setup = driver.random_setup(core::BattleSystem::Original);

            let mut ais = [
                ai::$mod::init($($arg,)* core::Player::Blue, &setup),
                ai::$mod::init($($arg,)* core::Player::Red, &setup),
            ];

            let mut active_ai = match setup.starting_player {
                core::Player::Blue => 0,
                core::Player::Red => 1,
            };

            driver.send(setup).unwrap();

            let mut res: Option<core::PlayOk> = None;
            'game_loop: loop {
                // battle to resolve
                res = if let Some(resolve) = res.and_then(|r| r.resolve_battle) {
                    let cmd = driver.resolve_battle(resolve);
                    ais[0].apply_resolve_battle(&cmd);
                    ais[1].apply_resolve_battle(&cmd);
                    Some(driver.send(cmd).unwrap())
                }
                // ai to move
                else {
                    let action = ais[active_ai].get_action();

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
                        core::Event::GameOver { .. } => {
                            break 'game_loop;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

benchmark!(expectiminimax_6_reduce_cloned_data, 4, 0.0);
benchmark!(expectiminimax_7_refactor, 4, 0.0);

iai::main!(
    expectiminimax_6_reduce_cloned_data,
    expectiminimax_7_refactor
);
