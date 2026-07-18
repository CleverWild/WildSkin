use super::msvc_string::MsvcString;

const VT_IS_LANE_MINION: usize = 0xF0;
// Additional minion-type vtable classifiers, reverse-engineered alongside
// `VT_IS_LANE_MINION` (the only one wired up today, in the minion skin path).
// Kept for finer jungle/epic/elite-minion skin filtering when that's wired.
#[allow(dead_code, reason = "additional minion-type classifiers, not yet wired — see comment above")]
const VT_IS_ELITE_MINION: usize = 0xF1;
#[allow(dead_code, reason = "additional minion-type classifiers, not yet wired — see comment above")]
const VT_IS_EPIC_MINION: usize = 0xF2;
#[allow(dead_code, reason = "additional minion-type classifiers, not yet wired — see comment above")]
const VT_IS_MINION: usize = 0xF6;
#[allow(dead_code, reason = "additional minion-type classifiers, not yet wired — see comment above")]
const VT_IS_JUNGLE: usize = 0xF7;

const NAME_OFFSET: usize = 0x68;
const TEAM_OFFSET: usize = 0x259;

#[repr(C)]
pub struct GameObject {
    _opaque: [u8; 0],
}

impl GameObject {
    /// # Safety
    /// Caller guarantees `self` points into a live `GameObject` in the
    /// game's memory, with a valid `MsvcString` at `NAME_OFFSET`.
    pub unsafe fn name(&self) -> &MsvcString {
        // SAFETY: per fn contract.
        unsafe {
            &*((std::ptr::from_ref::<Self>(self) as usize + NAME_OFFSET) as *const MsvcString)
        }
    }

    /// # Safety
    /// Caller guarantees `self` points into a live `GameObject` in the
    /// game's memory, with a valid `MsvcString` at `NAME_OFFSET`.
    pub unsafe fn name_mut(&mut self) -> &mut MsvcString {
        // SAFETY: per fn contract.
        unsafe {
            &mut *((std::ptr::from_mut::<Self>(self) as usize + NAME_OFFSET) as *mut MsvcString)
        }
    }

    /// # Safety
    /// Caller guarantees `self` points into a live `GameObject` in the
    /// game's memory, with a readable byte at `TEAM_OFFSET`.
    pub unsafe fn team(&self) -> i8 {
        // SAFETY: per fn contract.
        unsafe { *((std::ptr::from_ref::<Self>(self) as usize + TEAM_OFFSET) as *const i8) }
    }

    /// # Safety
    /// Caller guarantees `self` points into a live `GameObject` whose first
    /// pointer-sized field is a valid vtable pointer with at least
    /// `slot + 1` entries, and that slot `slot` holds a function pointer
    /// with signature `unsafe extern "system" fn(usize) -> bool`.
    unsafe fn call_virtual_bool(&self, slot: usize) -> bool {
        // SAFETY: per fn contract.
        let vtable = unsafe { *std::ptr::from_ref::<Self>(self).cast::<*const usize>() };
        // SAFETY: per fn contract. `add` indexes by pointer-sized slot,
        // matching the original's `vtable_ptr[Index]` — NOT a byte offset.
        let slot_ptr = unsafe { vtable.add(slot) };
        // SAFETY: per fn contract.
        let func_ptr = unsafe { *slot_ptr };
        // SAFETY: per fn contract.
        let func: unsafe extern "system" fn(usize) -> bool =
            unsafe { std::mem::transmute(func_ptr) };
        // SAFETY: per fn contract; `self`'s address is the implicit `this`.
        unsafe { func(std::ptr::from_ref::<Self>(self) as usize) }
    }

    /// # Safety
    /// Caller guarantees `self` is a valid `GameObject` with an intact
    /// vtable.
    pub unsafe fn is_lane_minion(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { self.call_virtual_bool(VT_IS_LANE_MINION) }
    }

    /// # Safety
    /// Caller guarantees `self` is a valid `GameObject` with an intact
    /// vtable.
    #[expect(dead_code, reason = "minion-type classifier, not yet wired — see the VT_IS_* comment")]
    pub unsafe fn is_elite_minion(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { self.call_virtual_bool(VT_IS_ELITE_MINION) }
    }

    /// # Safety
    /// Caller guarantees `self` is a valid `GameObject` with an intact
    /// vtable.
    #[expect(dead_code, reason = "minion-type classifier, not yet wired — see the VT_IS_* comment")]
    pub unsafe fn is_epic_minion(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { self.call_virtual_bool(VT_IS_EPIC_MINION) }
    }

    /// # Safety
    /// Caller guarantees `self` is a valid `GameObject` with an intact
    /// vtable.
    #[expect(dead_code, reason = "minion-type classifier, not yet wired — see the VT_IS_* comment")]
    pub unsafe fn is_minion(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { self.call_virtual_bool(VT_IS_MINION) }
    }

    /// # Safety
    /// Caller guarantees `self` is a valid `GameObject` with an intact
    /// vtable.
    #[expect(dead_code, reason = "minion-type classifier, not yet wired — see the VT_IS_* comment")]
    pub unsafe fn is_jungle(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { self.call_virtual_bool(VT_IS_JUNGLE) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_reads_the_byte_at_0x259() {
        let mut buf = [0u8; 0x260];
        buf[TEAM_OFFSET] = 2;
        let obj = unsafe { &*buf.as_ptr().cast::<GameObject>() };
        unsafe { assert_eq!(obj.team(), 2) };
    }

    #[test]
    fn vtable_call_indexes_by_pointer_sized_slots_not_byte_offset() {
        // Build a synthetic object: [vtable_ptr][...padding...], and a
        // synthetic vtable where slot VT_IS_LANE_MINION (240) is a function
        // that returns true. A byte-offset (rather than slot-index) bug
        // would read garbage a long way from this and misbehave loudly
        // (segfault) or silently (wrong bool) — this test pins the correct
        // arithmetic down.
        unsafe extern "system" fn returns_true(_this: usize) -> bool {
            true
        }

        let mut vtable = vec![0usize; VT_IS_JUNGLE + 1];
        vtable[VT_IS_LANE_MINION] = returns_true as *const () as usize;

        let vtable_ptr = vtable.as_ptr();
        let fake_object = [vtable_ptr as usize];
        let obj = unsafe { &*fake_object.as_ptr().cast::<GameObject>() };

        unsafe { assert!(obj.is_lane_minion()) };
    }
}
