pub mod settings_page;

use self::settings_page::{CanisterInfo, CanisterOwners, owners, settings_info};
use super::canisters::ic::add_controller;
use super::canisters::ledger::DefaultAccountAndBalance;
use super::stores::config::{handle_grant_ownership, ConfigStore, GrantOwnershipArgs, Status};
use super::stores::heap::StateStore;
use super::W3D_VERSION;
use crate::asset_certification::types::http::{
    CallbackFunc, HttpRequest, HttpResponse, StreamingCallbackHttpResponse, StreamingCallbackToken,
};
use crate::types::Permission;
use candid::{candid_method, Principal};
use ic_cdk::api::data_certificate;
use ic_cdk::{caller, query, trap, update};

#[update(guard = "can_commit")]
#[candid_method(update)]
async fn w3d_default_account_and_balance() -> DefaultAccountAndBalance {
    crate::web3disk::canisters::ledger::get_default_account_and_balance().await
}

#[update(guard = "can_commit")]
#[candid_method(update)]
async fn w3d_top_up() {
    crate::web3disk::interface::settings_page::top_up().await;
}

#[update(guard = "is_controller")]
#[candid_method(update)]
async fn w3d_grant_ownership(arg: GrantOwnershipArgs) {
    if ConfigStore::is_active() {
        trap("Already initialized")
    }

    handle_grant_ownership(arg).await;
}

// LOGIN
#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_status() -> Status {
    if caller() == Principal::anonymous() {
        trap("Anonymous principal not allowed")
    }

    ConfigStore::status()
}

#[query]
#[candid_method(query)]
async fn w3d_active() -> bool {
    if caller() == Principal::anonymous() {
        trap("Anonymous principal not allowed")
    }

    match ConfigStore::status() {
        Status::Active(_) => true,
        _ => false,
    }
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_api_version() -> String {
    W3D_VERSION.to_string()
}

#[update(guard = "can_commit")]
#[candid_method(update)]
pub async fn w3d_settings_info() -> CanisterInfo {
    settings_info().await
}

#[update(guard = "can_commit")]
#[candid_method(update)]
async fn w3d_owners() -> CanisterOwners {
    owners().await
}

#[update(guard = "is_controller")]
#[candid_method(update)]
async fn w3d_add_controller(p: Principal) {
    add_controller(p).await;
}

#[query(guard = "can_commit")]
#[candid_method(query)]
fn w3d_ii_principal() -> Principal {
    ConfigStore::ii_principal().unwrap_or_else(|| trap("No II principal set"))
}

#[query]
#[candid_method(query)]
fn http_request(req: HttpRequest) -> HttpResponse {
    let certificate = data_certificate().unwrap_or_else(|| trap("no data certificate available"));

    StateStore::http_request(
        req,
        &certificate,
        CallbackFunc::new(ic_cdk::id(), "http_request_streaming_callback".to_string()),
    )
}

#[query]
#[candid_method(query)]
fn http_request_streaming_callback(token: StreamingCallbackToken) -> StreamingCallbackHttpResponse {
    StateStore::http_request_streaming_callback(token)
}

fn can_commit() -> Result<(), String> {
    StateStore::can(Permission::Commit)
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
