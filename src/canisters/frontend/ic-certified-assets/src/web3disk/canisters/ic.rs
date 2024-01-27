use candid::Principal;
use ic_cdk::{
    api::management_canister::{
        main::{canister_status, update_settings, CanisterStatusResponse, UpdateSettingsArgument},
        provisional::{CanisterIdRecord, CanisterSettings},
    },
    trap,
};

pub async fn _canister_status() -> CanisterStatusResponse {
    let arg = CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    };

    canister_status(arg)
        .await
        .unwrap_or_else(|err| trap(&format!("{:?}", err)))
        .0
}

pub async fn _add_controller(p: Principal) {
    let mut settings = _canister_status().await.settings;

    settings.controllers.push(p);
    let new_controllers = settings.controllers;

    let arg = UpdateSettingsArgument {
        canister_id: ic_cdk::api::id(),
        settings: CanisterSettings {
            controllers: Some(new_controllers),
            memory_allocation: Some(settings.memory_allocation),
            compute_allocation: Some(settings.compute_allocation),
            freezing_threshold: Some(settings.freezing_threshold),
            reserved_cycles_limit: Some(settings.reserved_cycles_limit),
        },
    };

    update_settings(arg)
        .await
        .unwrap_or_else(|err| trap(&format!("{:?}, {:?}", err.0, err.1)));
}
