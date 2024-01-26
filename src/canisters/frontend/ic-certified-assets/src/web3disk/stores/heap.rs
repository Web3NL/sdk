use crate::{
    asset_certification::types::http::{
        CallbackFunc, HttpRequest, HttpResponse, StreamingCallbackHttpResponse,
        StreamingCallbackToken,
    },
    state_machine::State,
    types::{Permission, SetAssetPropertiesArguments, StoreArg},
};
use candid::Principal;
use ic_cdk::{caller, trap};
use ic_certification::Hash;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub struct StateStore;

impl StateStore {
    pub fn clear() {
        STATE.with(|s| s.borrow_mut().clear());
    }

    pub fn grant_permission(principal: Principal, permission: &Permission) {
        STATE.with(|s| s.borrow_mut().grant_permission(principal, permission));
    }

    pub fn store(arg: StoreArg, time: u64) -> Result<(), String> {
        STATE.with(|s| s.borrow_mut().store(arg, time))
    }

    pub fn set_asset_properties(arg: SetAssetPropertiesArguments) -> Result<(), String> {
        STATE.with(|s| s.borrow_mut().set_asset_properties(arg))
    }

    pub fn root_hash() -> Hash {
        STATE.with(|s| s.borrow().root_hash())
    }

    pub fn http_request(
        req: HttpRequest,
        certificate: &[u8],
        callback: CallbackFunc,
    ) -> HttpResponse {
        STATE.with(|s| s.borrow().http_request(req, certificate, callback))
    }

    pub fn http_request_streaming_callback(
        token: StreamingCallbackToken,
    ) -> StreamingCallbackHttpResponse {
        STATE.with(|s| {
            s.borrow()
                .http_request_streaming_callback(token)
                .unwrap_or_else(|msg| trap(&msg))
        })
    }

    pub fn can(permission: Permission) -> Result<(), String> {
        STATE.with(|s| {
            s.borrow()
                .can(&caller(), &permission)
                .then_some(())
                .ok_or_else(|| format!("Caller does not have {} permission", permission))
        })
    }
}
