pub mod api;
mod store;

use crate::asset_certification::types::rc_bytes::RcBytes;
use candid::{CandidType, Decode, Deserialize, Encode};
use ic_cdk::api::time;
use ic_stable_structures::{storable::Bound, Storable};
use std::{borrow::Cow, collections::HashMap, vec::IntoIter};

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
            return self.0.split('/').last().map(|name| name.to_string());
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
pub type Name = String;

#[derive(CandidType, Deserialize, Clone)]
enum Metadata {
    File(FileMetadata),
    Folder(FolderMetadata),
}

impl Metadata {
    pub fn new_root_folder_metadata() -> Self {
        Metadata::Folder(FolderMetadata {
            key: Key::new("/"),
            name: "__root__".to_string(),
            created: time(),
            last_modified: time(),
        })
    }
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

enum Node {
    File(Key),
    Folder(Key, HashMap<Name, Node>),
}
