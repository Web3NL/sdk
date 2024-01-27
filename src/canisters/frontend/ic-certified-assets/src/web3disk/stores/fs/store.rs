use super::{File, Key, Metadata};
use crate::web3disk::stores::{MemoryManagerStore, MEM_ID_FILE_DATA, MEM_ID_METADATA};
use candid::{Decode, Encode};
use ic_stable_structures::{
    memory_manager::VirtualMemory, storable::Bound, BTreeMap as StableBTree, DefaultMemoryImpl,
    Storable,
};
use std::{borrow::Cow, cell::RefCell};

thread_local! {
    static FILE_DATA: RefCell<StableBTree<Key, File, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_FILE_DATA))
    );

    static METADATA: RefCell<StableBTree<Key, Metadata, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_METADATA))
    );
}

struct FileDataStore;

impl FileDataStore {
    pub fn get(key: &Key) -> Option<File> {
        FILE_DATA.with(|refcell| refcell.borrow().get(key))
    }

    pub fn insert_new(key: &Key, file: File) -> Result<(), String> {
        FILE_DATA.with(|tree| {
            let mut tree = tree.borrow_mut();

            if tree.contains_key(&key) {
                return Err(format!("File already exists: {}", key.0));
            }

            if let None = tree.insert(key.clone(), file) {
                return Ok(());
            }

            Err("Failed to insert file".to_string())
        })
    }
}

struct MetadataStore;

impl MetadataStore {
    pub fn get(key: &Key) -> Option<Metadata> {
        METADATA.with(|refcell| refcell.borrow().get(key))
    }

    pub fn insert_new(key: &Key, metadata: &Metadata) -> Result<(), String> {
        METADATA.with(|tree| {
            let mut tree = tree.borrow_mut();

            if tree.contains_key(&key) {
                return Err(format!("Metadata already exists: {}", key.0));
            }

            if let None = tree.insert(key.clone(), metadata.clone()) {
                return Ok(());
            }

            Err("Failed to insert metadata".to_string())
        })
    }

    pub fn list_keys() -> Vec<Key> {
        METADATA.with(|tree| tree.borrow().iter().map(|(key, _)| key.clone()).collect())
    }
}

impl Storable for File {
    const BOUND: ic_stable_structures::storable::Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).expect("Failed to encode File"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).expect("Failed to decode File")
    }
}

impl Storable for Metadata {
    const BOUND: ic_stable_structures::storable::Bound = Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).expect("Failed to encode FileMetadata"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).expect("Failed to decode FileMetadata")
    }
}
