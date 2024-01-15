use super::ownership::{add_controller, handle_grant_ownership, GrantOwnershipArgs};
use super::state::{Status, W3DSTATE};
use super::STATE;
use crate::asset_certification::types::http::{
    CallbackFunc, HttpRequest, HttpResponse, StreamingCallbackHttpResponse, StreamingCallbackToken,
};
use crate::state_machine::{AssetDetails, EncodedAsset, State};
use crate::types::{DeleteAssetArguments, GetArg, Permission, StoreArg};
use candid::Principal;
use ic_cdk::api::management_canister::main::{canister_status, CanisterStatusResponse};
use ic_cdk::api::management_canister::provisional::CanisterIdRecord;
use ic_cdk::api::{data_certificate, set_certified_data, time};
use ic_cdk::{caller, query, trap, update};

const W3D_API_VERSION: &str = "0.0.1";

// State helper for frontend assets init, used in w3d_assets::init_frontend_assets()
pub fn assets_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|assets| f(&mut assets.borrow_mut()))
}

#[update(guard = "can_commit")]
fn w3d_store(arg: StoreArg) {
    STATE.with(move |s| {
        if let Err(msg) = s.borrow_mut().store(arg, time()) {
            trap(&msg);
        }
        set_certified_data(&s.borrow().root_hash());
    });
}

#[update(guard = "can_commit")]
fn w3d_delete_asset(arg: DeleteAssetArguments) {
    STATE.with(|s| {
        s.borrow_mut().delete_asset(arg);
        set_certified_data(&s.borrow().root_hash());
    });
}

#[query(guard = "can_commit")]
fn w3d_get(arg: GetArg) -> EncodedAsset {
    STATE.with(|s| match s.borrow().get(arg) {
        Ok(asset) => asset,
        Err(msg) => trap(&msg),
    })
}

#[query(guard = "can_commit")]
fn w3d_list() -> Vec<AssetDetails> {
    STATE
        .with(|s| s.borrow().list_assets())
        .into_iter()
        // .filter(|asset| asset.key.starts_with(W3D_ASSET_PREFIX))
        .collect()
}

#[update(guard = "is_controller")]
async fn w3d_grant_ownership(arg: GrantOwnershipArgs) {
    if W3DSTATE.with(|state| state.borrow().is_active()) {
        trap("Already initialized")
    }

    handle_grant_ownership(arg).await;
}

// LOGIN
#[query(guard = "can_commit")]
async fn w3d_status() -> Status {
    if caller() == Principal::anonymous() {
        trap("Anonymous principal not allowed")
    }

    W3DSTATE.with(|state| state.borrow().status())
}

#[query]
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
fn w3d_api_version() -> String {
    W3D_API_VERSION.to_string()
}

#[update(guard = "can_commit")]
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
async fn w3d_add_controller(p: Principal) {
    add_controller(p).await;
}

#[query(guard = "can_commit")]
fn w3d_ii_principal() -> Principal {
    W3DSTATE.with(|state| {
        state
            .borrow()
            .ii_principal()
            .unwrap_or_else(|| trap("No II principal set"))
    })
}

#[query]
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
