use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_stable_structures::{
    cell::Cell as StableCell,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    BTreeMap as StableBTree, DefaultMemoryImpl, Storable,
};
use std::{borrow::Cow, cell::RefCell};

static MEM_ID_W3DCONFIG: MemoryId = MemoryId::new(0);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static W3DCONFIG: RefCell<StableCell<W3DConfig, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|mm| mm.borrow().get(MEM_ID_W3DCONFIG)),
            W3DConfig::default()
        ).expect("Failed to init W3DConfig Stable Cell")
    );
}

pub struct W3DConfigStore;

impl W3DConfigStore {
    pub fn ii_principal() -> Option<Principal> {
        W3DCONFIG.with(|refcell| refcell.borrow().get().ii_principal())
    }

    pub fn set_ii_principal(ii_principal: Principal) {
        W3DCONFIG.with(|refcell| {
            let mut refcell = refcell.borrow_mut();
            let mut config = refcell.get().clone();

            config.set_ii_principal(ii_principal);
            refcell.set(config).expect("Failed to set ii_principal");
        });
    }

    pub fn status() -> Status {
        W3DCONFIG.with(|cell| cell.borrow().get().status())
    }

    pub fn set_status(status: Status) {
        W3DCONFIG.with(|refcell| {
            let mut refcell = refcell.borrow_mut();
            let mut config = refcell.get().clone();

            config.set_status(status);
            refcell.set(config).expect("Failed to set status");
        });
    }

    pub fn is_active() -> bool {
        W3DCONFIG.with(|refcell| refcell.borrow().get().is_active())
    }
}

#[derive(CandidType, Deserialize, Default, Clone, Copy)]
pub struct W3DConfig {
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

impl W3DConfig {
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

impl Storable for W3DConfig {
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
