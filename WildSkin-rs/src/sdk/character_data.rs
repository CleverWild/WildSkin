use super::primitives::AString;
use std::ffi::CStr;

// Game's MSVC `std::basic_string` destructor. A one-arg (`this`) thunk: test
// the heap-allocated flag, then tail-jump to the free path or `ret`. The
// trailing `E9 rel32` displacement is wildcarded, the rest is fixed opcodes.
#[abi_verify_macro::verify_abi(
    pattern = "F6 41 0C 01 74 08 48 8B 09",
    call_target = false,
    full_signature = "F6 41 0C 01 74 08 48 8B 09 E9 ? ? ? ? C3"
)]
pub(crate) type DtorFn = unsafe extern "system" fn(this: *mut AString);

// 16.14 layout (League of Legends.exe 16.14.794.5912): the only string is
// `model`; `~CharacterStackData()` frees just it. `_end` pins size to 0x90.
crate::offset!(
    pub struct CharacterStackData {
        0x00 => pub model: AString,
        0x20 => pub skin: i32,
        0x84 => pub gear: i8,
        0x8f => _end: u8,
    }
);

impl CharacterStackData {
    /// Mirrors the game's `~CharacterStackData()`: frees the one `model` string.
    ///
    /// # Safety
    /// Caller guarantees `self` is a live, fully-constructed
    /// `CharacterStackData` and `dtor_fn` is the game's own MSVC string
    /// destructor.
    unsafe fn destroy_strings(&mut self, dtor_fn: DtorFn) {
        // SAFETY: per fn contract.
        unsafe {
            dtor_fn(&raw mut self.model);
        }
    }
}

#[repr(C)]
struct MsvcVector<T> {
    begin: *mut T,
    end: *mut T,
    cap_end: *mut T,
}

impl<T> MsvcVector<T> {
    unsafe fn is_empty(&self) -> bool {
        self.begin == self.end
    }

    unsafe fn front_mut(&mut self) -> &mut T {
        // SAFETY: caller guarantees the vector is non-empty and `begin` is valid.
        unsafe { &mut *self.begin }
    }

    const unsafe fn clear(&mut self) {
        self.end = self.begin;
    }
}

#[repr(C)]
pub struct CharacterDataStack {
    stack: MsvcVector<CharacterStackData>,
    pub base_skin: CharacterStackData,
}

#[abi_verify_macro::verify_abi(
    pattern = "E8 ? ? ? ? 48 8D 8D ? ? 00 00 E8 ? ? ? ? 48 85 C0 74 ? 48 85 ED",
    // Push's real 16.14 prologue (spills, non-volatile pushes, `lea rbp,[rsp-7]`,
    // `sub rsp,0xD0`). No rel32 here, so no wildcards. Pins the frame size,
    // which is what moved when the arg list grew to 18.
    full_signature = "48 89 5C 24 10 48 89 74 24 18 48 89 7C 24 20 55 41 54 41 55 41 56 41 57 48 8D 6C 24 F9 48 81 EC D0 00 00 00 48 8B DA"
)]
// `this`/`model`/`skin`/`gear` confirmed; each `unknown_*`/`flag_*` is a
// placeholder for a slot whose purpose wasn't pinned down (position/rough type
// only). `str1` is last in the arg list but writes element `+0x10`; the game
// strlen's it, so omitting it crashes on the garbage stack slot.
pub(crate) type PushFn = unsafe extern "system" fn(
    this: usize,
    model: *const i8,
    skin: i32,
    unknown_i32_1: i32,
    flag_1: bool,
    flag_2: bool,
    flag_3: bool,
    unknown_flag_lookup: bool,
    flag_4: bool,
    flag_5: bool,
    gear: i8,
    str2: *const i8,
    unknown_i32_2: i32,
    str4: *const i8,
    unknown_i32_3: i32,
    unknown_flag_post: bool,
    unknown_i32_4: i32,
    str1: *const i8,
) -> i64;

#[abi_verify_macro::verify_abi(
    pattern = "88 54 24 10 55 53 56 57 41 54 41 55 41 56 41",
    call_target = false,
    // First 28 bytes of Update's real 16.14 prologue (non-volatile pushes,
    // `lea rbp,[rsp-0x1f]`, `sub rsp,0x88`). No relative displacements, no wildcards.
    full_signature = "88 54 24 10 55 53 56 57 41 54 41 55 41 56 41 57 48 8D 6C 24 E1 48 81 EC 88 00 00 00"
)]
pub(crate) type UpdateFn = unsafe extern "system" fn(this: usize, change: bool) -> i64;

/// The three resolved game functions a skin change drives (push a stack entry,
/// update the stack, free a discarded entry's strings), always passed together.
pub(crate) struct SkinApplyFns {
    pub push: PushFn,
    pub update: UpdateFn,
    pub dtor: DtorFn,
}

impl CharacterDataStack {
    pub unsafe fn is_stack_empty(&self) -> bool {
        // SAFETY: caller guarantees `stack`'s begin/end pointers are valid.
        unsafe { self.stack.is_empty() }
    }

    pub unsafe fn stack_front_mut(&mut self) -> &mut CharacterStackData {
        // SAFETY: caller guarantees the stack is non-empty and valid.
        unsafe { self.stack.front_mut() }
    }

    #[allow(
        dead_code,
        reason = "plain (leaky) stack clear, faithful to the original C++ and exercised by unit tests; the skin path uses `clear_stack_properly` instead"
    )]
    pub const unsafe fn clear_stack(&mut self) {
        // SAFETY: caller guarantees `stack`'s begin/end pointers are valid.
        unsafe { self.stack.clear() }
    }

    /// Like `clear_stack` but first calls `destroy_strings` on every discarded
    /// element, matching the game's `std::vector<CharacterStackData>::clear()`.
    /// `clear_stack`'s plain begin/end reset skips this and leaks each element's
    /// string fields; `change_skin`'s clear+push uses this to avoid leaking.
    ///
    /// # Safety
    /// Caller guarantees `self.stack`'s begin/end pointers are valid and
    /// `dtor_fn` is the game's own MSVC string destructor.
    pub unsafe fn clear_stack_properly(&mut self, dtor_fn: DtorFn) {
        let mut cursor = self.stack.begin;
        while cursor != self.stack.end {
            // SAFETY: per fn contract; every slot in [begin, end) is live.
            let slot = unsafe { &mut *cursor };
            // SAFETY: per fn contract.
            unsafe { slot.destroy_strings(dtor_fn) };
            // SAFETY: `cursor` stays within [begin, end) until the loop ends.
            cursor = unsafe { cursor.add(1) };
        }
        self.stack.end = self.stack.begin;
    }

    pub unsafe fn push(&self, push_fn: PushFn, model: &CStr, skin: i32) {
        let empty = c"".as_ptr();
        // SAFETY: caller guarantees `self` is a live `CharacterDataStack`.
        unsafe {
            push_fn(
                std::ptr::from_ref::<Self>(self) as usize,
                model.as_ptr(),
                skin,
                0,
                false,
                false,
                false,
                false,
                true,
                false,
                -1,
                empty,
                0,
                empty,
                0,
                false,
                1,
                empty,
            );
        }
    }

    pub unsafe fn update(&self, update_fn: UpdateFn, change: bool) {
        // SAFETY: caller guarantees `self` is a live `CharacterDataStack`.
        unsafe {
            update_fn(std::ptr::from_ref::<Self>(self) as usize, change);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    fn zeroed_stack_data() -> CharacterStackData {
        // A zeroed AString is this type's valid empty state (see
        // `AString::as_str`'s null check).
        // SAFETY: `CharacterStackData` is `#[repr(C)]` and POD-shaped, so
        // all-zero is valid for every field.
        unsafe { std::mem::zeroed() }
    }

    #[test]
    fn character_stack_data_is_0x90_bytes_matching_the_games_grow_and_emplace() {
        assert_eq!(size_of::<CharacterStackData>(), 0x90);
    }

    #[test]
    fn empty_vector_reports_empty() {
        let mut base = zeroed_stack_data();
        let cds = CharacterDataStack {
            stack: MsvcVector {
                begin: &raw mut base,
                end: &raw mut base,
                cap_end: &raw mut base,
            },
            base_skin: zeroed_stack_data(),
        };
        unsafe { assert!(cds.is_stack_empty()) };
    }

    #[test]
    fn nonempty_vector_reports_not_empty_and_front_is_readable() {
        let mut items = [zeroed_stack_data()];
        items[0].skin = 42;
        let end_ptr = unsafe { items.as_mut_ptr().add(1) };
        let mut cds = CharacterDataStack {
            stack: MsvcVector {
                begin: items.as_mut_ptr(),
                end: end_ptr,
                cap_end: end_ptr,
            },
            base_skin: zeroed_stack_data(),
        };
        unsafe {
            assert!(!cds.is_stack_empty());
            assert_eq!(cds.stack_front_mut().skin, 42);
        }
    }

    #[test]
    fn clear_makes_it_empty_again() {
        let mut items = [zeroed_stack_data()];
        let end_ptr = unsafe { items.as_mut_ptr().add(1) };
        let mut cds = CharacterDataStack {
            stack: MsvcVector {
                begin: items.as_mut_ptr(),
                end: end_ptr,
                cap_end: end_ptr,
            },
            base_skin: zeroed_stack_data(),
        };
        unsafe {
            assert!(!cds.is_stack_empty());
            cds.clear_stack();
            assert!(cds.is_stack_empty());
        }
    }

    #[test]
    fn push_and_update_call_through_to_a_stub_matching_the_original_signature() {
        // Stubs mimicking the Win64 game functions with the 18-param arg list
        // (see `PushFn`). A mismatched arg count/type is the #1 way this layer
        // segfaults against the real game, so exercise the call path.
        static PUSH_SKIN: AtomicI32 = AtomicI32::new(0);
        static PUSH_EXTRA: AtomicI32 = AtomicI32::new(0);
        static PUSH_LAST_N3: AtomicI32 = AtomicI32::new(0);
        static PUSH_STR1_EMPTY: AtomicBool = AtomicBool::new(false);
        static UPDATE_CHANGE: AtomicBool = AtomicBool::new(false);

        unsafe extern "system" fn stub_push(
            _this: usize,
            _model: *const i8,
            skin: i32,
            extra: i32,
            _b1: bool,
            _b2: bool,
            _b3: bool,
            _b4: bool,
            _b5: bool,
            _b6: bool,
            _gear: i8,
            _s1: *const i8,
            _n1: i32,
            _s2: *const i8,
            _n2: i32,
            _b7: bool,
            n3: i32,
            str1: *const i8,
        ) -> i64 {
            PUSH_SKIN.store(skin, Ordering::SeqCst);
            PUSH_EXTRA.store(extra, Ordering::SeqCst);
            PUSH_LAST_N3.store(n3, Ordering::SeqCst);
            // SAFETY: `push` passes `c""`; non-null is checked before reading.
            PUSH_STR1_EMPTY.store(!str1.is_null() && unsafe { *str1 } == 0, Ordering::SeqCst);
            0
        }

        unsafe extern "system" fn stub_update(_this: usize, change: bool) -> i64 {
            UPDATE_CHANGE.store(change, Ordering::SeqCst);
            0
        }

        let cds = CharacterDataStack {
            stack: MsvcVector {
                begin: std::ptr::null_mut(),
                end: std::ptr::null_mut(),
                cap_end: std::ptr::null_mut(),
            },
            base_skin: zeroed_stack_data(),
        };
        let model = std::ffi::CString::new("Ahri").unwrap();

        unsafe {
            cds.push(stub_push, &model, 99);
            cds.update(stub_update, true);
        }

        assert_eq!(PUSH_SKIN.load(Ordering::SeqCst), 99);
        assert_eq!(PUSH_EXTRA.load(Ordering::SeqCst), 0);
        // Checking both ends guards the arg list staying complete and aligned.
        assert_eq!(PUSH_LAST_N3.load(Ordering::SeqCst), 1);
        assert!(
            PUSH_STR1_EMPTY.load(Ordering::SeqCst),
            "18th arg must arrive as a readable empty string"
        );
        assert!(UPDATE_CHANGE.load(Ordering::SeqCst));
    }
}
