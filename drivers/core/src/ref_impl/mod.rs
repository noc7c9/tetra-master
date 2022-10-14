use crate::{
    self as core, command,
    response::{self, ErrorResponse},
    CommandResponse, Error,
};

mod logic;

pub type ReferenceImplementation = State;

pub enum State {
    PreSetup,
    InGame(logic::State),
}

impl State {
    pub fn new() -> Self {
        Self::PreSetup
    }

    pub fn step<C: Step>(&mut self, cmd: C) -> core::Result<C::Response> {
        match cmd.step(self) {
            Err(err) => Err(Error::ErrorResponse(err)),
            Ok(ok) => Ok(ok),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Step: CommandResponse {
    fn step(self, state: &mut State) -> Result<Self::Response, ErrorResponse>;
}

impl Step for command::Setup {
    fn step(self, state: &mut State) -> Result<Self::Response, ErrorResponse> {
        if let State::PreSetup = state {
            *state = State::InGame(logic::State::new(&self));

            Ok(response::SetupOk {
                blocked_cells: self.blocked_cells,
                battle_system: self.battle_system,
                hand_blue: self.hand_blue,
                hand_red: self.hand_red,
                starting_player: self.starting_player,
            })
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PlaceCard {
    fn step(self, state: &mut State) -> Result<Self::Response, ErrorResponse> {
        if let State::InGame(ref mut state) = state {
            state.handle_place_card(self)
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::PickBattle {
    fn step(self, state: &mut State) -> Result<Self::Response, ErrorResponse> {
        if let State::InGame(ref mut state) = state {
            state.handle_pick_battle(self)
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}

impl Step for command::ResolveBattle {
    fn step(self, state: &mut State) -> Result<Self::Response, ErrorResponse> {
        if let State::InGame(ref mut state) = state {
            state.handle_resolve_battle(self)
        } else {
            panic!("Unexpected command {self:?}")
        }
    }
}
