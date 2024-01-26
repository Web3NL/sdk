use crate::{
    asset_certification::types::rc_bytes::RcBytes,
    web3disk::stores::{MemoryManagerStore, MEM_ID_FILE_DATA, MEM_ID_METADATA},
};
use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::{
    memory_manager::VirtualMemory, storable::Bound, BTreeMap as StableBTree, DefaultMemoryImpl,
    Storable,
};
use std::{borrow::Cow, cell::RefCell, vec::IntoIter};

#[derive(CandidType, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub struct Key(String);

impl Key {
    pub fn new(key: &str) -> Self {
        Self(key.to_string())
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn is_file(&self) -> bool {
        !self.0.ends_with('/')
    }

    pub fn is_dir(&self) -> bool {
        self.0.ends_with('/')
    }

    pub fn file_name(&self) -> Option<String> {
        if self.is_file() {
            return Some(self.0.split('/').last().expect("No file name").to_string());
        }
        None
    }

    pub fn iter_dir_names(&self) -> IntoIter<String> {
        let mut path = self
            .0
            .split('/')
            .map(|dir| dir.to_string())
            .collect::<Vec<String>>();

        if self.is_file() {
            path.pop();
        }

        path.into_iter()
    }
}

impl Storable for Key {
    const BOUND: ic_stable_structures::storable::Bound = Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).expect("Failed to encode Key"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).expect("Failed to decode Key")
    }
}

pub type File = RcBytes;

#[derive(CandidType, Deserialize, Clone)]
enum Metadata {
    File(FileMetadata),
    Folder(FolderMetadata),
}

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

#[derive(CandidType, Deserialize, Clone)]
struct FolderMetadata {
    pub key: Key,
    pub name: String,
    pub created: u64,
    pub last_modified: u64,
}

pub type DirEntries = Vec<DirEntry>;

enum DirEntry {
    File(Key),
    Dir(DirEntries),
}

thread_local! {
    static FILE_DATA: RefCell<StableBTree<Key, File, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_FILE_DATA))
    );

    static METADATA: RefCell<StableBTree<Key, Metadata, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTree::init(MemoryManagerStore::get(MEM_ID_METADATA))
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
                return Err(format!("File already exists: {}", key.0));
            }

            if let Some(_) = tree.insert(key.clone(), file) {
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

    pub fn insert(key: &Key, metadata: &Metadata) -> Result<(), String> {
        METADATA.with(|tree| {
            let mut tree = tree.borrow_mut();

            if tree.contains_key(&key) {
                return Err(format!("Metadata already exists: {}", key.0));
            }

            if let Some(_) = tree.insert(key.clone(), metadata.clone()) {
                return Ok(());
            }

            Err("Failed to insert metadata".to_string())
        })
    }

    // pub fn file_tree_root() -> Result<DirEntries, String> {}
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
