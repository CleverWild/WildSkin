#[repr(C)]
pub struct AString {
    pub(crate) ptr: *const i8,
    pub(crate) length: i32,
    pub(crate) capacity: i32,
}

impl AString {
    pub unsafe fn as_str(&self) -> &str {
        if self.ptr.is_null() {
            return "";
        }
        // SAFETY: caller guarantees `ptr` points at a valid, NUL-terminated
        // C string for the lifetime of `&self` (non-null checked above).
        let cstr = unsafe { std::ffi::CStr::from_ptr(self.ptr) };
        cstr.to_str().unwrap_or("")
    }
}

#[repr(C)]
pub struct RiotArray<T> {
    list: *mut T,
    size: i32,
    cap: i32,
}

impl<T> RiotArray<T> {
    pub const unsafe fn as_slice(&self) -> &[T] {
        if self.list.is_null() || self.size <= 0 {
            return &[];
        }
        // SAFETY: caller guarantees `list` points to `size` valid,
        // initialized `T` elements for the lifetime of `&self` (non-null
        // and size > 0 checked above).
        unsafe { std::slice::from_raw_parts(self.list, self.size as usize) }
    }
}

crate::offset!(
    pub struct ManagerTemplate<T> {
        0x8  => list: *mut *mut T,
        0x10 => length: i32,
        0x14 => capacity: i32,
    }
);

impl<T> ManagerTemplate<T> {
    pub const unsafe fn as_slice(&self) -> &[*mut T] {
        if self.list.is_null() || self.length <= 0 {
            return &[];
        }
        // SAFETY: caller guarantees `list` points to `length` valid
        // `*mut T` pointer entries for the lifetime of `&self` (non-null
        // and length > 0 checked above).
        unsafe { std::slice::from_raw_parts(self.list, self.length as usize) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn astring_reads_through_the_pointer() {
        let backing = CString::new("Ahri").unwrap();
        let a = AString { ptr: backing.as_ptr(), length: 4, capacity: 4 };
        unsafe { assert_eq!(a.as_str(), "Ahri"); }
    }

    #[test]
    fn astring_null_pointer_is_empty_not_a_crash() {
        let a = AString { ptr: std::ptr::null(), length: 0, capacity: 0 };
        unsafe { assert_eq!(a.as_str(), ""); }
    }

    #[test]
    fn riot_array_reads_backing_slice() {
        let mut backing = [1i32, 2, 3];
        let arr = RiotArray { list: backing.as_mut_ptr(), size: 3, cap: 3 };
        unsafe { assert_eq!(arr.as_slice(), &[1, 2, 3]) };
    }

    #[test]
    fn manager_template_reads_pointer_list_and_skips_padding() {
        let mut a = 10i32;
        let mut b = 20i32;
        let mut list = [&raw mut a, &raw mut b];
        let mt = ManagerTemplate::<i32> {
            _padlist: [0; 8],
            list: list.as_mut_ptr(),
            _padlength: [0; 0],
            length: 2,
            _padcapacity: [0; 0],
            capacity: 2,
        };
        let slice = unsafe { mt.as_slice() };
        assert_eq!(slice.len(), 2);
        unsafe { assert_eq!(*slice[0], 10); assert_eq!(*slice[1], 20); }
    }
}
