mod api;
mod assets;
pub mod frontend;
mod ownership;
mod state;

use crate::state_machine::State;
use crate::types::Permission;
use assets::init_frontend_assets;
use std::cell::RefCell;

thread_local! {
    pub static STATE: RefCell<State> = RefCell::new(State::default());
}

#[ic_cdk::init]
pub fn init() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.clear();
        s.grant_permission(ic_cdk::caller(), &Permission::Commit);
    });

    // Web3Disk init frontend in `STATE` thread local storage
    init_frontend_assets();
}

#[ic_cdk::post_upgrade]
pub fn post_upgrade() {
    init();
}
