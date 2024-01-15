use candid::{CandidType, Deserialize, Principal};
use std::cell::RefCell;

thread_local! {
    pub static W3DSTATE: RefCell<W3DState> = RefCell::new(W3DState::new());
}

#[derive(Clone, Debug, CandidType)]
pub struct W3DState {
    pub status: Status,
    pub ii_principal: Option<Principal>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Default)]
pub enum Status {
    #[default]
    Setup,
    Active(Mode),
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub enum Mode {
    Developer,
    Trial,
    User,
}

impl W3DState {
    pub fn new() -> Self {
        Self {
            status: Status::Setup,
            ii_principal: None,
        }
    }

    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn ii_principal(&self) -> Option<Principal> {
        self.ii_principal.clone()
    }

    pub fn set_ii_principal(&mut self, ii_principal: Principal) {
        self.ii_principal = Some(ii_principal);
    }

    pub fn is_active(&self) -> bool {
        match self.status() {
            Status::Active(_) => true,
            _ => false,
        }
    }
}
