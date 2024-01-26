mod assets;
mod frontend;
mod interface;
mod ownership;
mod stores;

use crate::types::Permission;
use assets::init_frontend_assets;

use self::stores::{config::W3DConfigStore, heap::StateStore};

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

    if let Some(ii_principal) = W3DConfigStore::ii_principal() {
        StateStore::grant_permission(ii_principal, &Permission::Commit);
    }
}
