use crate::{
    asset_certification::types::{certification::AssetKey, rc_bytes::RcBytes},
    web3disk::stores::{MemoryManagerStore, MEM_ID_FILE_DATA, MEM_ID_FILE_METADATA},
};
use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::{
    memory_manager::VirtualMemory, storable::Bound, BTreeMap as StableBTree, DefaultMemoryImpl,
    Storable,
};
use std::{borrow::Cow, cell::RefCell};

pub type Key = AssetKey;
pub type File = RcBytes;

#[derive(CandidType, Deserialize, Clone)]
pub struct FileMetadata {
    pub key: Key,
    pub name: String,
    pub extension: String,
    pub content_type: String,
    pub size: u64,
    pub created: u64,
    pub last_modified: u64,
    pub last_accessed: u64,
    pub sha256: Vec<u8>,
}

thread_local! {
    static FILE_DATA: RefCell<StableBTree<Key, File, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_FILE_DATA))
    );

    static FILE_METADATA: RefCell<StableBTree<Key, FileMetadata, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_FILE_METADATA))
    );
}

struct FileData;

impl FileData {
    pub fn get(key: &Key) -> Option<File> {
        FILE_DATA.with(|refcell| refcell.borrow().get(key))
    }

    pub fn insert(key: &Key, file: File) -> Result<(), String> {
        FILE_DATA.with(|tree| {
            let mut tree = tree.borrow_mut();

            if tree.contains_key(&key) {
                return Err(format!("File already exists: {}", key));
            }

            if let Some(_) = tree.insert(key.clone(), file) {
                return Ok(());
            }

            Err("Failed to insert file".to_string())
        })
    }
}

struct FileMetadataData;

impl FileMetadataData {
    pub fn get(key: &Key) -> Option<FileMetadata> {
        FILE_METADATA.with(|refcell| refcell.borrow().get(key))
    }

    pub fn insert(key: &Key, metadata: &FileMetadata) -> Result<(), String> {
        FILE_METADATA.with(|tree| {
            let mut tree = tree.borrow_mut();

            if tree.contains_key(&key) {
                return Err(format!("FileMetadata already exists: {}", key));
            }

            if let Some(_) = tree.insert(key.to_string(), metadata.clone()) {
                return Ok(());
            }

            Err("Failed to insert file metadata".to_string())
        })
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

impl Storable for FileMetadata {
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
