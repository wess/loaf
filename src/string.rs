//! JavaScript string handle.

use std::ptr;

use crate::error::{Error, Result};
use crate::handle::Ref;
use crate::sys;

/// An owned handle to a JavaScript string.
///
/// JS strings are UTF-16 internally; loaf exchanges them as UTF-8 at the
/// boundary. Reading copies into an owned Rust `String`.
#[derive(Clone)]
pub struct JsString(pub(crate) Ref);

impl JsString {
    /// Copy the string's contents into an owned Rust `String`.
    pub fn to_str(&self) -> Result<String> {
        String::from_utf8(self.bytes()?).map_err(|_| Error::Utf8)
    }

    /// Copy the string's UTF-8 bytes into an owned `Vec`.
    pub fn bytes(&self) -> Result<Vec<u8>> {
        let mut out = sys::LoafBytes {
            ptr: ptr::null(),
            len: 0,
        };
        // Safety: handle and out-param are valid for this call.
        let st = unsafe { sys::loaf_get_string(self.0.rt_raw(), self.0.raw, &mut out) };
        match st {
            sys::LoafStatus::Ok => {
                // Safety: on Ok, libloaf hands back a valid ptr/len it owns.
                let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
                unsafe { sys::loaf_bytes_free(out) };
                Ok(bytes)
            }
            sys::LoafStatus::Utf8 => Err(Error::Utf8),
            _ => Err(Error::Runtime("failed to read JS string".into())),
        }
    }
}

impl std::fmt::Debug for JsString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsString").finish_non_exhaustive()
    }
}
