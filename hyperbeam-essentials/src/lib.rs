#![feature(proc_macro_hygiene)]
#![feature(asm)]

use hyperbeam_rtdx::modpack::{ModpackMetadata, MODPACK_BASE_PATH};
use hyperbeam_unity::{reflect, texture_helpers, IlString};
use lazy_static;
use pmdrtdx_bindings::*;
use skyline::nn;
use skyline::{hook, install_hook, install_hooks};
use std::ffi::{CString, c_void};
use std::io::Read;
use std::os::raw::c_char;
use std::ptr::{self, null_mut};
use std::string::String;
use std::slice;
use flate2::read::DeflateDecoder;

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

        // TODO: fix!!!!!!!!!!!!!!!!! (currently None)
        let id = "techticks.testhack";//&MODPACK.as_ref().unwrap().id;
        let new_path = format!("{}/{}/romfs/{}", MODPACK_BASE_PATH, id, &original_path[5..]);
        println!("[hyperbeam-essentials] Trying to load: {}", new_path);
        let new_path_cstring = CString::new(new_path).unwrap();
        let res = call_original!(handle, new_path_cstring.as_ptr(), mode);
        if res == 0 {
            0
        } else {
            println!(
                "[hyperbeam-essentials] Failed to load file, falling back to: {}",
                original_path
            );
            call_original!(handle, path, mode)
        }
    } else {
        call_original!(handle, path, mode)
    }

    // TODO: how does the game check if save data exists? does it just try to open the file?
}

// TODO: add to symbol map
#[hook(offset = 0x264B650)]
unsafe fn hook_native_decompress_gyu0(output: *mut u8, input: *const u8, unk1: i32, unk2: *mut c_void, unk3: *mut c_void) -> i32 {
    // Allow deflate compression in addition to the "GYU0" format used by the game
    if *input as char == 'D' && *input.offset(1) as char == 'E' && *input.offset(2) as char == 'F' && *input.offset(3) as char == 'L' {
        // deflate compression
        let decompressed_size = *(input.offset(4) as *const u32);
        let compressed_size = *(input.offset(8) as *const u32);

        let in_slice = slice::from_raw_parts(input.offset(12), compressed_size as usize);
        let out_slice = slice::from_raw_parts_mut(output, decompressed_size as usize);
        let mut decoder = DeflateDecoder::new(in_slice);
        decoder.read_exact(out_slice).unwrap();
        0
    } else {
        // GYU0 compression, use original path
        call_original!(output, input, unk1, unk2, unk3)
    }
}

#[skyline::main(name = "hyperbeam_essentials")]
pub fn main() {
    println!(
        "[hyperbeam-essentials] Initializing for modpack: {:?}",
        *MODPACK
    );

    println!("[hyperbeam-essentials] Installing file hooks...");
    install_hooks!(hook_open_file, hook_native_decompress_gyu0);
}
