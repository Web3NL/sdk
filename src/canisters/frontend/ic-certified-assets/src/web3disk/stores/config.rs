use super::heap::StateStore;
use crate::{
    types::Permission,
    web3disk::{
        canisters::ic::canister_status,
        stores::{MemoryManagerStore, MEM_ID_CONFIG},
    },
};
use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_cdk::{
    api::management_canister::{
        main::{update_settings, UpdateSettingsArgument},
        provisional::CanisterSettings,
    },
    trap,
};
use ic_stable_structures::{
    cell::Cell as StableCell, memory_manager::VirtualMemory, storable::Bound, DefaultMemoryImpl,
    Storable,
};
use std::{borrow::Cow, cell::RefCell};

thread_local! {
    static CONFIG: RefCell<StableCell<Config, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableCell::init(
            MemoryManagerStore::get(MEM_ID_CONFIG),
            Config::default()
        ).expect("Failed to init Config Stable Cell")
    );
}

pub struct ConfigStore;

impl ConfigStore {
    pub fn ii_principal() -> Option<Principal> {
        CONFIG.with(|refcell| refcell.borrow().get().ii_principal())
    }

    pub fn set_ii_principal(ii_principal: Principal) {
        CONFIG.with(|refcell| {
            let mut refcell = refcell.borrow_mut();
            let mut config = refcell.get().clone();

            config.set_ii_principal(ii_principal);
            refcell.set(config).expect("Failed to set ii_principal");
        });
    }

    pub fn status() -> Status {
        CONFIG.with(|cell| cell.borrow().get().status())
    }

    pub fn set_status(status: Status) {
        CONFIG.with(|refcell| {
            let mut refcell = refcell.borrow_mut();
            let mut config = refcell.get().clone();

            config.set_status(status);
            refcell.set(config).expect("Failed to set status");
        });
    }

    pub fn is_active() -> bool {
        CONFIG.with(|refcell| refcell.borrow().get().is_active())
    }
}

#[derive(CandidType, Deserialize, Default, Clone, Copy)]
struct Config {
    pub status: Status,
    pub ii_principal: Option<Principal>,
}

#[derive(CandidType, Deserialize, Default, Clone, Copy)]
pub enum Status {
    #[default]
    Setup,
    Active(Mode),
}

#[derive(CandidType, Deserialize, Clone, Debug, Copy)]
pub enum Mode {
    Developer,
    Trial,
    User,
}

impl Config {
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn ii_principal(&self) -> Option<Principal> {
        self.ii_principal.clone()
    }

    pub fn set_ii_principal(&mut self, ii_principal: Principal) {
        self.ii_principal = Some(ii_principal);
    }

    pub fn is_active(&self) -> bool {
        match self.status() {
            Status::Active(_) => true,
            _ => false,
        }
    }
}

impl Storable for Config {
    const BOUND: Bound = Bound::Bounded {
        max_size: 256,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }
}

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

            let mut settings = canister_status().await.settings;

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

            update_settings(update_settings_arg)
                .await
                .unwrap_or_else(|err| trap(&format!(" {:?}, {} ", err.0, err.1)));

            ConfigStore::set_ii_principal(args.ii_principal);
            ConfigStore::set_status(Status::Active(args.mode));
        }
    }
}

fn grant_commit_permission(p: Principal) {
    ConfigStore::set_ii_principal(p);
    StateStore::grant_permission(p, &Permission::Commit);
}
