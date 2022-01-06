use pmdrtdx_bindings as unity;
use std::error::Error;
use std::fmt;
use std::ptr;

use super::*;

#[derive(Debug)]
pub enum LoadAssetError {
    BundleNotLoaded,
    AssetNotFound(String),
}

impl Error for LoadAssetError {}

impl fmt::Display for LoadAssetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadAssetError::BundleNotLoaded => write!(f, "Asset bundle not loaded"),
            LoadAssetError::AssetNotFound(name) => write!(f, "Asset not found: {}", name),
        }
    }
}

pub struct AssetBundleWrapper {
    bundle: Option<*mut unity::AssetBundle>,
}

impl AssetBundleWrapper {
    pub fn new() -> AssetBundleWrapper {
        AssetBundleWrapper { bundle: None }
    }

    pub fn load_from_full_path(&mut self, path: &str) {
        let result = unsafe { unity::AssetBundle_LoadFromFile(
            IlString::new(path).as_ptr(),
            ptr::null_mut(),
        )};
        self.bundle = if result.is_null() {
            None
        } else {
            Some(result)
        };
    }

    pub fn load_from_file(&mut self, name: &str) {
        self.load_from_full_path(&format!("rom:/Data/StreamingAssets/ab/{}.ab", name))
    }

    pub fn unload(&mut self, unload_all_loaded_objects: bool) {
        if let Some(bundle) = self.bundle {
            unsafe {
                unity::AssetBundle_Unload(bundle, unload_all_loaded_objects, ptr::null_mut());
            }
            self.bundle = None;
        }
    }

    pub fn load_asset(
        &self,
        name: &str,
        il_type: &unity::Type,
    ) -> Result<*mut UnityObject, LoadAssetError> {
        if let Some(bundle) = self.bundle {
            let asset = unsafe {
                unity::AssetBundle_LoadAsset(
                    bundle,
                    IlString::new(name).as_ptr(),
                    il_type as *const unity::Type as *mut unity::Type,
                    ptr::null_mut(),
                )
            };
            if asset.is_null() {
                Err(LoadAssetError::AssetNotFound(name.to_owned()))
            } else {
                Ok(asset)
            }
        } else {
            Err(LoadAssetError::BundleNotLoaded)
        }
    }
}
