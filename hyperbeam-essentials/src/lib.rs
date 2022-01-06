#![feature(proc_macro_hygiene)]
#![feature(asm)]

use pmdrtdx_bindings::*;
use std::ptr::{null_mut};
use skyline::{hook, install_hook, install_hooks};
use skyline::nn;
use hyperbeam_unity::{IlString, reflect, texture_helpers};
use hyperbeam_rtdx::modpack::{ModpackMetadata, MODPACK_BASE_PATH};
use std::ffi::{CString};
use std::os::raw::c_char;
use std::string::String;
use lazy_static;

lazy_static::lazy_static! {
    static ref MODPACK: Option<ModpackMetadata> = unsafe { hbGetCurrentModpackMetadata() };
}

extern "Rust" {
    fn hbGetCurrentModpackMetadata() -> Option<ModpackMetadata>;
}

#[hook(replace = nn::fs::OpenFile)]
unsafe fn hook_open_file(handle: *mut nn::fs::FileHandle, path: *const c_char, mode: i32) -> i32 {
    let original_path = std::ffi::CStr::from_ptr(path).to_str().unwrap();
    if original_path.starts_with("rom:/") {
        let id = &MODPACK.as_ref().unwrap().id;
        let new_path = format!("{}/{}/romfs/{}", MODPACK_BASE_PATH, id, &original_path[5..]);
        println!("[hyperbeam-essentials] Trying to load: {}", new_path);
        let new_path_cstring = CString::new(new_path).unwrap();
        let res = call_original!(handle, new_path_cstring.as_ptr(), mode);
        if res == 0 {
            0
        } else {
            println!("[hyperbeam-essentials] Failed to load file, falling back to: {}", original_path);
            call_original!(handle, path, mode)
        }
    } else {
        call_original!(handle, path, mode)
    }

    // TODO: how does the game check if save data exists? does it just try to open the file?
}

#[skyline::main(name = "hyperbeam_essentials")]
pub fn main() {
    println!("[hyperbeam-essentials] Initializing for modpack: {:?}", *MODPACK);

    println!("[hyperbeam-essentials] Installing file hooks...");
    install_hook!(hook_open_file);
}
