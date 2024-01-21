mod api;
mod assets;
pub mod frontend;
mod ownership;
mod state;

use crate::state_machine::{StableState, State};
use crate::types::{AssetCanisterArgs, InitArgs, Permission, UpgradeArgs};
use assets::init_frontend_assets;
use ic_cdk::api::set_certified_data;
use ic_cdk::caller;
use std::cell::RefCell;

thread_local! {
    pub static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn init(args: Option<AssetCanisterArgs>) {
    if let Some(upgrade_arg) = args {
        let AssetCanisterArgs::Init(InitArgs {}) = upgrade_arg else {
            ic_cdk::trap("Cannot initialize the canister with an Upgrade argument. Please provide an Init argument.")
        };
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.clear();
        s.grant_permission(caller(), &Permission::Commit);
    });

    // Web3Disk init frontend in `STATE` thread local storage
    init_frontend_assets();
}

pub fn pre_upgrade() -> StableState {
    STATE.with(|s| s.take().into())
}

pub fn post_upgrade(stable_state: StableState, args: Option<AssetCanisterArgs>) {
    let set_permissions = args.and_then(|args| {
        let AssetCanisterArgs::Upgrade(UpgradeArgs { set_permissions }) = args else {ic_cdk::trap("Cannot upgrade the canister with an Init argument. Please provide an Upgrade argument.")};
        set_permissions
    });

    STATE.with(|s| {
        *s.borrow_mut() = State::from(stable_state);
        set_certified_data(&s.borrow().root_hash());
        if let Some(set_permissions) = set_permissions {
            s.borrow_mut().set_permissions(set_permissions);
        }
    });
}
