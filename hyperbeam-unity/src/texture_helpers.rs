use super::*;
use std::ptr;
use std::ptr::{null_mut, NonNull};
use std::{error::Error, fmt};

pub fn texture2d_from_bytes(
    bytes: &Vec<u8>,
    width: i32,
    height: i32,
    generate_mip_maps: bool,
    readable: bool,
) -> NonNull<unity::Texture2D> {
    unsafe {
        let size = (width * height * 4) as usize;
        assert_eq!(
            bytes.len(),
            size,
            "Number of bytes doesn't match texture dimensions"
        );

        let tex =
            unity::il2cpp_object_new(unity::Texture2D__TypeInfo as _) as *mut unity::Texture2D;
        unity::Texture2D__ctor_2(
            tex,
            width,
            height,
            unity::TextureFormat__Enum_RGBA32,
            generate_mip_maps,
            null_mut(),
        );
        let color32_type = reflect::get_unity_type(Some("UnityEngine"), "Color32").unwrap();
        let color_array = unity::Array_CreateInstance_1(color32_type, width * height, null_mut())
            as *mut unity::Il2CppArraySize;
        assert!(!color_array.is_null(), "Failed to create IL array");

        let array = (*color_array).vector.as_mut_ptr() as *mut u8;
        ptr::copy_nonoverlapping(bytes.as_ptr(), array, size);

        unity::Texture2D_SetPixels32(tex, color_array as _, 0, null_mut());
        unity::Texture2D_Apply(tex, generate_mip_maps, !readable, null_mut());

        tex.as_mut().expect("Failed to create texture").into()
    }
}
