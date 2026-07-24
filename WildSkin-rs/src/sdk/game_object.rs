use super::msvc_string::MsvcString;

// `name`/`team` are struct fields; the `vtable` block declares virtual-call
// wrappers (vtable pointer at offset 0). Gaps between fields are padding.
crate::offset!(
    pub struct GameObject {
        0x68 => pub name: MsvcString,
        0x259 => pub team: i8,
    }
    vtable {
        /// Calls the game's `isLaneMinion` virtual. Slot 0xF0 is RE'd and NOT
        /// abi-verified (a slot index has no AOB), so it can drift between
        /// patches; suspect this first if lane-minion skins break after an
        /// update. Elite/epic/generic/jungle slots (0xF1/0xF2/0xF6/0xF7) were
        /// RE'd too but dropped as unused.
        0xF0 => pub fn is_lane_minion(&self) -> bool;
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_reads_the_byte_at_0x259() {
        // `GameObject` is 8-aligned (usize fields), so the backing buffer must
        // be too; a plain `[u8]` is only 1-aligned.
        #[repr(align(8))]
        struct Aligned([u8; 0x260]);
        let mut buf = Aligned([0u8; 0x260]);
        buf.0[0x259] = 2;
        // SAFETY: `buf` is 8-aligned and 0x260 bytes, covering `GameObject`; `team` reads 0x259.
        let obj = unsafe { &*std::ptr::from_ref(&buf).cast::<GameObject>() };
        assert_eq!(obj.team, 2);
    }

    #[test]
    fn vtable_call_indexes_by_pointer_sized_slots_not_byte_offset() {
        // Synthetic object [vtable_ptr][padding] + synthetic vtable where slot
        // 0xF0 returns true. Pins the slot-index (not byte-offset) arithmetic:
        // a byte-offset bug would read garbage and segfault or return a wrong bool.
        unsafe extern "system" fn returns_true(_this: usize) -> bool {
            true
        }

        let mut vtable = vec![0usize; 0xF0 + 1];
        vtable[0xF0] = returns_true as *const () as usize;

        let vtable_ptr = vtable.as_ptr();
        // Back `GameObject` with enough storage; only the vtable pointer at
        // offset 0 is read here.
        let mut fake_object = vec![0usize; size_of::<GameObject>() / size_of::<usize>() + 1];
        fake_object[0] = vtable_ptr as usize;
        // SAFETY: `fake_object` covers `GameObject`; only offset 0 (vtable) is read.
        let obj = unsafe { &*fake_object.as_ptr().cast::<GameObject>() };

        unsafe { assert!(obj.is_lane_minion()) };
    }
}
