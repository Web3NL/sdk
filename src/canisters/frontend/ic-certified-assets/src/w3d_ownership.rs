use crate::w3d_state::{Mode, Status, W3DSTATE};
use crate::Permission;
use crate::{assets_mut, w3d_canister_status};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::{
    management_canister::main::{update_settings, CanisterSettings, UpdateSettingsArgument},
    trap,
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

            let mut settings = w3d_canister_status().await.settings;

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
                        },
                    }
                }
                _ => trap("Impossible mode error"),
            };

            match update_settings(update_settings_arg).await {
                Ok(_) => {}
                Err(err) => trap(&format!(" {:?}, {} ", err.0, err.1)),
            }

            W3DSTATE.with(|state| {
                state.borrow_mut().set_ii_principal(args.ii_principal);
                state.borrow_mut().set_status(Status::Active(args.mode));
            });
        }
    }
}

pub async fn add_controller(p: Principal) {
    let mut settings = w3d_canister_status().await.settings;

    settings.controllers.push(p);
    let new_controllers = settings.controllers;

    let arg = UpdateSettingsArgument {
        canister_id: ic_cdk::api::id(),
        settings: CanisterSettings {
            controllers: Some(new_controllers),
            memory_allocation: Some(settings.memory_allocation),
            compute_allocation: Some(settings.compute_allocation),
            freezing_threshold: Some(settings.freezing_threshold),
        },
    };

    match update_settings(arg).await {
        Ok(_) => {}
        Err(err) => trap(&format!(" {:?}, {} ", err.0, err.1)),
    }
}

fn grant_commit_permission(p: Principal) {
    assets_mut(|s| {
        s.grant_permission(p, &Permission::Commit);
    });
}
