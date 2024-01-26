// use crate::state::{Mode, Status, W3DSTATE};
// use crate::Permission;
// use crate::{assets_mut, canister_status};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::{
    management_canister::{
        main::{canister_status, update_settings, CanisterSettings, UpdateSettingsArgument, CanisterStatusResponse},
        provisional::CanisterIdRecord,
    },
    trap,
};

use crate::types::Permission;

use super::{
    api::assets_mut,
    state::{Mode, Status, W3DConfigStore},
};

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct GrantOwnershipArgs {
    mode: Mode,
    ii_principal: Principal,
}

pub async fn handle_grant_ownership(args: GrantOwnershipArgs) {
    // Web3Disk Modes
    // Developer: Grant caller (II Principal) commit permission and add as controller
    // Trial: Grant caller (II Principal) commit permission
    // User: Grant caller (II Principal) commit permission and set as only controller
    // besides canister principal itself

    match args.mode {
        Mode::Trial => grant_commit_permission(args.ii_principal),
        Mode::Developer | Mode::User => {
            grant_commit_permission(args.ii_principal);
            
            let mut settings = canister_status_response().await.settings;

            let update_settings_arg: UpdateSettingsArgument = match args.mode {
                Mode::Developer => {
                    // We only add the II principal as an additional controller, keeping dev IDs
                    settings.controllers.push(args.ii_principal);

                    UpdateSettingsArgument {
                        canister_id: ic_cdk::api::id(),
                        settings: CanisterSettings {
                            controllers: Some(settings.controllers),
                            memory_allocation: Some(settings.memory_allocation),
                            compute_allocation: Some(settings.compute_allocation),
                            freezing_threshold: Some(settings.freezing_threshold),
                            reserved_cycles_limit: Some(settings.reserved_cycles_limit),
                        },
                    }
                }
                Mode::User => {
                    // We set the II principal and canister id as the only controllers
                    let controllers = vec![ic_cdk::api::id(), args.ii_principal];

                    UpdateSettingsArgument {
                        canister_id: ic_cdk::api::id(),
                        settings: CanisterSettings {
                            controllers: Some(controllers),
                            memory_allocation: Some(settings.memory_allocation),
                            compute_allocation: Some(settings.compute_allocation),
                            freezing_threshold: Some(settings.freezing_threshold),
                            reserved_cycles_limit: Some(settings.reserved_cycles_limit),
                        },
                    }
                }
                _ => trap("Impossible mode error"),
            };

            match update_settings(update_settings_arg).await {
                Ok(_) => {}
                Err(err) => trap(&format!(" {:?}, {} ", err.0, err.1)),
            }

            W3DConfigStore::set_ii_principal(args.ii_principal);
            W3DConfigStore::set_status(Status::Active(args.mode));
        }
    }
}

pub async fn add_controller(p: Principal) {
    let mut settings = canister_status_response().await.settings;

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

    match update_settings(arg).await {
        Ok(_) => {}
        Err(err) => trap(&format!(" {:?}, {} ", err.0, err.1)),
    }
}

fn grant_commit_permission(p: Principal) {
    W3DConfigStore::set_ii_principal(p);
    
    assets_mut(|s| {
        s.grant_permission(p, &Permission::Commit);
    });
}

pub async fn canister_status_response() -> CanisterStatusResponse {
    let arg = CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    };

    canister_status(arg).await
        .unwrap_or_else(|err| trap(&format!("{:?}", err)))
        .0
}