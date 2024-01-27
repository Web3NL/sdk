use crate::{
    types::{SetAssetPropertiesArguments, StoreArg},
    web3disk::stores::heap::StateStore,
};
use ic_cdk::api::{set_certified_data, time, trap};
use include_dir::{include_dir, Dir};
use mime::Mime;
use serde_bytes::ByteBuf;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

static ASSET_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../../../../src/web3disk/build");

const MAX_CHUNK_SIZE: usize = 1000 * 1024;

struct Asset {
    pub key: String,
    pub data: Vec<u8>,
    pub media_type: Mime,
}

pub fn init_frontend_assets() {
    let assets = collect_assets_recursive(&ASSET_DIR);

    for mut asset in assets {
        if asset.data.len() > MAX_CHUNK_SIZE {
            let msg = format!(
                "Asset too large: {} ({} bytes)",
                asset.key,
                asset.data.len()
            );
            trap(msg.as_str());
        }

        // */index.html hook
        if asset.key.ends_with("/index.html") {
            asset.key = asset.key.replace("/index.html", "/");
        }

        // *.html hook
        if asset.key.ends_with(".html") {
            asset.key = asset.key.replace(".html", "");
        }

        let store_arg = StoreArg {
            key: asset.key,
            content_type: asset.media_type.to_string(),
            content_encoding: "identity".to_string(),
            sha256: Some(ByteBuf::from(Sha256::digest(&asset.data).to_vec())),
            content: ByteBuf::from(asset.data),
            aliased: Some(false),
        };

        StateStore::store(store_arg, time()).unwrap_or_else(|_| trap("store failed"));

        set_certified_data(&StateStore::root_hash());
    }

    // add CORS to /.well-known/ii-alternative-origins
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert(
        "Access-Control-Allow-Origin".to_string(),
        // "https://identity.internetcomputer.org".to_string(),
        "*".to_string(),
    );
    map.insert("Content-Type".to_string(), "application/json".to_string());

    let arg: SetAssetPropertiesArguments = SetAssetPropertiesArguments {
        headers: Some(Some(map)),
        key: "/.well-known/ii-alternative-origins".to_string(),
        max_age: None,
        is_aliased: None,
        allow_raw_access: None,
    };

    StateStore::set_asset_properties(arg).unwrap_or_else(|_| trap("set_asset_properties failed"));

    set_certified_data(&StateStore::root_hash());
}

fn collect_assets_recursive(dir: &Dir) -> Vec<Asset> {
    let mut assets = collect_assets_from_dir(dir);
    for subdir in dir.dirs() {
        assets.extend(collect_assets_recursive(subdir).into_iter());
    }
    assets
}

fn collect_assets_from_dir(dir: &Dir) -> Vec<Asset> {
    let mut assets: Vec<Asset> = vec![];
    for asset in dir.files() {
        let path = asset.path().to_str().unwrap().to_string();
        let key = format!("/{}", path);

        let data = asset.contents().to_vec();

        // todo: check contents if mime_guess fails https://github.com/dfinity/sdk/issues/1594
        let media_type = mime_guess::from_path(asset.path())
            .first()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);

        assets.push(Asset {
            key,
            data,
            media_type,
        });
    }
    assets
}
