use super::*;
use std::ptr;
use std::ffi::c_void;

fn create_assembly_qualified_name(namespace_name: Option<&str>, type_name: &str, assembly_name: &str) -> IlString {
    if let Some(namespace_name) = namespace_name {
        IlString::new(format!("{}.{}, {}", namespace_name, type_name, assembly_name))
    } else {
        IlString::new(format!("{}, {}", type_name, assembly_name))
    }
}

pub fn get_type<'a>(namespace_name: Option<&'a str>, type_name: &'a str, assembly_name: &'a str) -> Option<&'a mut unity::Type> {
    let assembly_qualified_name = create_assembly_qualified_name(namespace_name, type_name, assembly_name);
    let pointer = unsafe { unity::Type_GetType_2(assembly_qualified_name.as_ptr(), ptr::null_mut()) };
    unsafe { pointer.as_mut() }
}

pub fn get_assembly_csharp_type<'a>(namespace_name: Option<&'a str>, type_name: &'a str) -> Option<&'a mut unity::Type> {
    get_type(namespace_name, type_name, "Assembly-CSharp")
}

pub fn get_unity_type<'a>(namespace_name: Option<&'a str>, type_name: &'a str) -> Option<&'a mut unity::Type> {
    get_type(namespace_name, type_name, "UnityEngine")
}

pub fn get_unity_ui_type<'a>(namespace_name: Option<&'a str>, type_name: &'a str) -> Option<&'a mut unity::Type> {
    get_type(namespace_name, type_name, "UnityEngine.UI")
}
