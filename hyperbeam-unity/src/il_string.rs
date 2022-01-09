use pmdrtdx_bindings as unity;
use std::fmt::{Display, Formatter};
use std::ptr;
use std::slice;

pub struct IlString {
    string: *mut unity::String,
}

impl IlString {
    pub fn new<T: AsRef<str>>(string: T) -> IlString {
        let mut utf16_string: Vec<u16> = string.as_ref().encode_utf16().collect();
        utf16_string.push(0); // Seems like things go crazy without a null terminator although the length is passed
        IlString {
            string: unsafe {
                unity::String_CreateString_3(
                    ptr::null_mut(),
                    utf16_string.as_mut_ptr(),
                    0,
                    string.as_ref().len() as _,
                    ptr::null_mut(),
                )
            },
        }
    }

    pub fn as_ptr(&self) -> *mut unity::String {
        return self.string;
    }

    pub fn is_null(&self) -> bool {
        return self.string.is_null();
    }

    pub fn len(&self) -> i32 {
        return unsafe { self.string.as_ref() }.unwrap().m_stringLength;
    }
}

impl From<*mut unity::String> for IlString {
    fn from(string: *mut unity::String) -> IlString {
        IlString { string }
    }
}

impl Display for IlString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if !self.string.is_null() {
            let il2cpp_string = self.string as *mut unity::Il2CppString;
            let slice = unsafe {
                slice::from_raw_parts(
                    (*il2cpp_string).chars.as_mut_ptr(),
                    (*il2cpp_string).length as _,
                )
            };
            let string = String::from_utf16(slice).expect("Invalid UTF-16!");
            f.write_str(string.as_str())
        } else {
            f.write_str("(null)")
        }
    }
}
