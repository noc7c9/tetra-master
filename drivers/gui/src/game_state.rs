use tetra_master_core::{self as core, Command, Response};

pub mod picking_hands {
    use super::*;

    pub struct State {
        driver: core::Driver,
        pub candidates: [Option<core::Hand>; 3],
        pub status: Status,
    }

    pub enum Status {
        BluePicking,
        RedPicking {
            hand_blue: core::Hand,
        },
        Done {
            hand_blue: core::Hand,
            hand_red: core::Hand,
        },
    }

    impl State {
        pub fn new(implementation: &str) -> Self {
            let mut driver = core::Driver::new(implementation).log();
            if let Response::SetupOk {
                hand_candidates, ..
            } = driver
                .send(Command::Setup {
                    rng: None,
                    battle_system: None,
                    blocked_cells: None,
                    hand_candidates: None,
                })
                // TODO: handle error instead of panic
                .unwrap()
            {
                Self {
                    driver,
                    candidates: [
                        Some(hand_candidates[0]),
                        Some(hand_candidates[1]),
                        Some(hand_candidates[2]),
                    ],
                    status: Status::BluePicking {},
                }
            } else {
                unreachable!()
            }
        }

        pub fn picking_player(&self) -> core::Player {
            match &self.status {
                Status::BluePicking { .. } => core::Player::P1,
                Status::RedPicking { .. } => core::Player::P2,
                Status::Done { .. } => {
                    unreachable!("Both hands already picked")
                }
            }
        }

        pub fn pick_hand(&mut self, hand: usize) {
            match &self.status {
                Status::BluePicking => {
                    let response = self
                        .driver
                        .send(core::Command::PickHand { hand: hand as u8 })
                        .expect("PickHand command should work");
                    // TODO: expose expect_pick_hand_ok() method from tester crate
                    if !matches!(response, core::Response::PickHandOk) {
                        panic!("PickHand command should work");
                    }

                    self.status = Status::RedPicking {
                        hand_blue: self.candidates[hand]
                            .take()
                            .expect("pick index should be correct"),
                    }
                }
                Status::RedPicking { hand_blue } => {
                    let response = self
                        .driver
                        .send(core::Command::PickHand { hand: hand as u8 })
                        .expect("PickHand command should work");
                    if !matches!(response, core::Response::PickHandOk) {
                        panic!("PickHand command should work");
                    }

                    self.status = Status::Done {
                        hand_blue: *hand_blue,
                        hand_red: self.candidates[hand]
                            .take()
                            .expect("pick index should be correct"),
                    }
                }
                Status::Done { .. } => {
                    unreachable!("Both hands already picked")
                }
            }
        }
    }
}
