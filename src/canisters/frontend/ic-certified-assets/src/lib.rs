// WEB3DISK MODIFICATIONS NOTICE
// lib.rs is the only file modified.
// All other files are in sync with the original source code.
// Updates are merged from master regularly.
//
// Test module succesfully runs tests, since the asset canister implementation remains unchanged.
//
// Additional Web3Disk filenames start with `w3d_` prefix.
// Use git diff to see the changes.

pub mod asset_certification;
pub mod evidence;
pub mod state_machine;
pub mod types;
mod url_decode;
mod w3d_assets;
mod w3d_ownership;
mod w3d_state;

#[cfg(test)]
mod tests;

pub use crate::state_machine::StableState;
use crate::{
    asset_certification::types::http::{
        CallbackFunc, HttpRequest, HttpResponse, StreamingCallbackHttpResponse,
        StreamingCallbackToken,
    },
    state_machine::{AssetDetails, EncodedAsset, State},
    types::*,
};
use candid::{candid_method, Principal};
use ic_cdk::api::management_canister::main::{
    canister_status, CanisterIdRecord, CanisterStatusResponse,
};
use ic_cdk::api::{caller, data_certificate, set_certified_data, time, trap};
use ic_cdk::{query, update};
use std::cell::RefCell;

use w3d_assets::init_frontend_assets;
use w3d_ownership::{add_controller, handle_grant_ownership, GrantOwnershipArgs};
use w3d_state::{Status, W3DSTATE};

#[cfg(target_arch = "wasm32")]
#[link_section = "icp:public supported_certificate_versions"]
pub static SUPPORTED_CERTIFICATE_VERSIONS: [u8; 3] = *b"1,2";

const W3D_API_VERSION: &str = "0.0.1";
pub const W3D_ASSET_PREFIX: &str = "__w3d__";

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

// State helper for frontend assets init, used in w3d_assets::init_frontend_assets()
pub fn assets_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|assets| f(&mut assets.borrow_mut()))
}

#[update(guard = "can_commit")]
#[candid_method(update)]
fn w3d_store(arg: StoreArg) {
    STATE.with(move |s| {
        if let Err(msg) = s.borrow_mut().store(arg, time()) {
            trap(&msg);
        }
        set_certified_data(&s.borrow().root_hash());
    });
}

#[update(guard = "can_commit")]
#[candid_method(update)]
fn w3d_delete_asset(arg: DeleteAssetArguments) {
    // if !arg.key.starts_with(W3D_ASSET_PREFIX) {
    //     trap("Cannot delete assets without __w3d__ path prefix")
    // }

    STATE.with(|s| {
        s.borrow_mut().delete_asset(arg);
        set_certified_data(&s.borrow().root_hash());
    });
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_get(arg: GetArg) -> EncodedAsset {
    // if !arg.key.starts_with(W3D_ASSET_PREFIX) {
    //     trap("Cannot get assets without __w3d__ path prefix")
    // }

    STATE.with(|s| match s.borrow().get(arg) {
        Ok(asset) => asset,
        Err(msg) => trap(&msg),
    })
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_list() -> Vec<AssetDetails> {
    STATE
        .with(|s| s.borrow().list_assets())
        .into_iter()
        // .filter(|asset| asset.key.starts_with(W3D_ASSET_PREFIX))
        .collect()
}

#[update(guard = "is_controller")]
#[candid_method(update)]
async fn w3d_grant_ownership(arg: GrantOwnershipArgs) {
    if W3DSTATE.with(|state| state.borrow().is_active()) {
        trap("Already initialized")
    }

    handle_grant_ownership(arg).await;
}

// LOGIN
#[query(guard = "can_commit")]
#[candid_method(query)]
async fn w3d_status() -> Status {
    if caller() == Principal::anonymous() {
        trap("Anonymous principal not allowed")
    }

    W3DSTATE.with(|state| state.borrow().status())
}

#[query]
#[candid_method(query)]
async fn w3d_active() -> bool {
    if caller() == Principal::anonymous() {
        trap("Anonymous principal not allowed")
    }

    match W3DSTATE.with(|state| state.borrow().status()) {
        Status::Active(_) => true,
        _ => false,
    }
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_api_version() -> String {
    W3D_API_VERSION.to_string()
}

#[update(guard = "can_commit")]
#[candid_method(update)]
pub async fn w3d_canister_status() -> CanisterStatusResponse {
    let arg = CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    };

    match canister_status(arg).await {
        Ok(status) => return status.0,
        Err(err) => trap(&format!("{:?}, {}", err.0, err.1)),
    }
}

#[update(guard = "is_controller")]
#[candid_method(update)]
async fn w3d_add_controller(p: Principal) {
    add_controller(p).await;
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_ii_principal() -> Principal {
    W3DSTATE.with(|state| {
        state
            .borrow()
            .ii_principal()
            .unwrap_or_else(|| trap("No II principal set"))
    })
}

#[query]
#[candid_method(query)]
fn http_request(req: HttpRequest) -> HttpResponse {
    let certificate = data_certificate().unwrap_or_else(|| trap("no data certificate available"));

    STATE.with(|s| {
        s.borrow().http_request(
            req,
            &certificate,
            CallbackFunc::new(ic_cdk::id(), "http_request_streaming_callback".to_string()),
        )
    })
}

#[query]
#[candid_method(query)]
fn http_request_streaming_callback(token: StreamingCallbackToken) -> StreamingCallbackHttpResponse {
    STATE.with(|s| {
        s.borrow()
            .http_request_streaming_callback(token)
            .unwrap_or_else(|msg| trap(&msg))
    })
}

fn can(permission: Permission) -> Result<(), String> {
    STATE.with(|s| {
        s.borrow()
            .can(&caller(), &permission)
            .then_some(())
            .ok_or_else(|| format!("Caller does not have {} permission", permission))
    })
}

fn can_commit() -> Result<(), String> {
    can(Permission::Commit)
}

fn is_controller() -> Result<(), String> {
    let caller = caller();
    if ic_cdk::api::is_controller(&caller) {
        Ok(())
    } else {
        Err("Caller is not a controller.".to_string())
    }
}

// pub fn init() {
//     STATE.with(|s| {
//         let mut s = s.borrow_mut();
//         s.clear();
//         s.grant_permission(caller(), &Permission::Commit);
//     });

//     // Web3Disk init frontend in `STATE` thread local storage
//     init_frontend_assets();
// }

// pub fn pre_upgrade() -> StableState {
//     STATE.with(|s| s.take().into())
// }

// pub fn post_upgrade(stable_state: StableState) {
//     STATE.with(|s| {
//         *s.borrow_mut() = State::from(stable_state);
//         set_certified_data(&s.borrow().root_hash());
//     });
// }

#[test]
fn generate_candid() {
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    const DIST_DIR: &str = "../../../../../src/distributed/web3disk";
    const WEB3DISK_DID: &str = "web3disk.did";

    candid::export_service!();
    let web3disk = __export_service();

    File::create(
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join(DIST_DIR)
            .join(WEB3DISK_DID)
            .as_path(),
    )
    .unwrap()
    .write_all(&web3disk.as_bytes())
    .expect("Unable to write candid file");
}

// use crate::{
//     asset_certification::types::http::{
//         CallbackFunc, HttpRequest, HttpResponse, StreamingCallbackHttpResponse,
//         StreamingCallbackToken,
//     },
//     state_machine::{AssetDetails, CertifiedTree, EncodedAsset, State},
//     types::*,
// };
// use asset_certification::types::{certification::AssetKey, rc_bytes::RcBytes};
// use candid::{candid_method, Principal};
// use ic_cdk::api::{call::ManualReply, caller, data_certificate, set_certified_data, time, trap};
// use ic_cdk::{query, update};
// use serde_bytes::ByteBuf;
// use std::cell::RefCell;

pub fn init(args: Option<AssetCanisterArgs>) {
    if let Some(upgrade_arg) = args {
        let AssetCanisterArgs::Init(InitArgs {}) = upgrade_arg else { ic_cdk::trap("Cannot initialize the canister with an Upgrade argument. Please provide an Init argument.")};
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.clear();
        s.grant_permission(caller(), &Permission::Commit);
    });
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
