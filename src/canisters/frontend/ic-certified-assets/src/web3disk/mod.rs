mod canisters;
mod frontend;
mod interface;
mod stores;

use self::{
    frontend::assets::init_frontend_assets,
    stores::{config::ConfigStore, heap::StateStore},
};
use crate::types::Permission;

pub const W3D_VERSION: &str = "0.0.2";

#[ic_cdk::init]
pub fn init() {
    StateStore::clear();
    StateStore::grant_permission(ic_cdk::caller(), &Permission::Commit);

    // Init frontend dir in `STATE` thread local storage
    init_frontend_assets();
}

#[ic_cdk::post_upgrade]
pub fn post_upgrade() {
    init();

    if let Some(ii_principal) = ConfigStore::ii_principal() {
        StateStore::grant_permission(ii_principal, &Permission::Commit);
    }
}
