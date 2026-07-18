//! MSVC `std::string` ABI mirror, used for exactly one field: the player's
//! editable display name (`GameObject::Name` in the original C++). MSVC's
//! `std::string` uses small-string-optimization (SSO): strings up to 15
//! bytes live inline in a 16-byte buffer; longer strings heap-allocate
//! through the game's own CRT allocator. Reading works for either case;
//! writing is SSO-only (see `set_sso`).

const SSO_CAPACITY: usize = 15;

#[repr(C)]
pub struct MsvcString {
    buf: [u8; 16],
    size: usize,
    capacity: usize,
}

impl MsvcString {
    /// # Safety
    /// Caller guarantees `self` is a valid, initialized MSVC `std::string`:
    /// if `capacity >= 16`, the first 8 bytes of `buf` are a live pointer to
    /// `size` initialized bytes (heap case); otherwise `buf` itself holds
    /// `size` initialized bytes inline (SSO case).
    pub unsafe fn as_str(&self) -> &str {
        let ptr = if self.capacity < 16 {
            self.buf.as_ptr()
        } else {
            #[allow(clippy::cast_ptr_alignment, reason = "buf is 8-byte aligned transitively via the struct's usize fields forcing 8-byte alignment — verified during this struct's original review")]
            let heap_ptr = self.buf.as_ptr().cast::<*const u8>();
            // SAFETY: caller guarantees the first 8 bytes of `buf` are a
            // valid heap pointer when `capacity >= 16`.
            unsafe { *heap_ptr }
        };
        // SAFETY: caller guarantees `ptr` points to `size` valid,
        // initialized bytes for the lifetime of `&self`.
        let slice = unsafe { std::slice::from_raw_parts(ptr, self.size) };
        std::str::from_utf8(slice).unwrap_or("")
    }

    /// ponytail: SSO-only; grow-to-heap path unsupported, see module doc.
    ///
    /// # Safety
    /// Caller guarantees `self` is a valid, initialized MSVC `std::string`.
    pub unsafe fn set_sso(&mut self, value: &str) -> bool {
        if self.capacity >= 16 || value.len() > SSO_CAPACITY {
            return false;
        }
        self.buf[..value.len()].copy_from_slice(value.as_bytes());
        self.buf[value.len()] = 0;
        self.size = value.len();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_sso() -> MsvcString {
        MsvcString { buf: [0; 16], size: 0, capacity: 15 }
    }

    #[test]
    fn sso_roundtrip() {
        let mut s = empty_sso();
        unsafe {
            assert!(s.set_sso("Renekton"));
            assert_eq!(s.as_str(), "Renekton");
        }
    }

    #[test]
    fn rejects_names_too_long_for_sso() {
        let mut s = empty_sso();
        unsafe { assert!(!s.set_sso("ThisNameIsWayTooLongForSSO")) };
    }

    #[test]
    fn refuses_to_touch_an_already_heap_allocated_string() {
        let mut s = MsvcString { buf: [0; 16], size: 20, capacity: 31 };
        unsafe { assert!(!s.set_sso("short")) };
    }

    #[test]
    fn reads_heap_allocated_string_through_the_pointer() {
        use std::ffi::CString;
        let backing = CString::new("a very long summoner name here").unwrap();
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&(backing.as_ptr() as usize).to_le_bytes());
        let s = MsvcString { buf, size: backing.as_bytes().len(), capacity: 31 };
        unsafe { assert_eq!(s.as_str(), "a very long summoner name here") };
    }
}
