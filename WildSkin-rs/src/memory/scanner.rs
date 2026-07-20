pub struct Signature {
    pub patterns: &'static [&'static str],
    pub sub_base: bool,
    pub read: bool,
    pub relative: bool,
    pub additional: i32,
}

fn find_first(haystack: &[u8], pattern: &str) -> Option<usize> {
    let builder = aobscan::PatternBuilder::from_ida_style(pattern).ok()?;
    // Single-threaded, matching the reference's `find_signature` exactly.
    // `with_all_threads()` spawns num_cpus::get() OS threads per call, which
    // saturates every logical core on the host for the scan's duration —
    // across the 14 sequential signatures resolved at injection time, that
    // starved the live game process of CPU on every core and was the root
    // cause of a multi-second freeze right after injection. A single-threaded
    // linear scan over a multi-MB .text section still completes in well
    // under a frame's worth of time.
    let scanner = builder.with_threads(1).ok()?.build();
    let mut found = None;
    scanner.scan(haystack, |offset| {
        found = Some(offset);
        false // false = stop; the reference returns the FIRST match, not the last
    });
    found
}

/// Byte offset of the pattern's first wildcard token. Mirrors the original's
/// `pattern.find_first_of('?') / 3` — each token is 2 hex chars + 1 space.
#[allow(clippy::integer_division, reason = "exact division by the fixed 3-char token width, not an approximation")]
fn first_wildcard_byte_offset(pattern: &str) -> usize {
    pattern.find('?').expect("read signature must contain a wildcard") / 3
}

pub unsafe fn resolve(base: usize, module: &[u8], sig: &Signature) -> Option<usize> {
    for pattern in sig.patterns {
        let Some(offset) = find_first(module, pattern) else { continue };
        // `module` is the `.text` section slice, which starts at `base +
        // VirtualAddress`, not at `base` itself (see `text_section`) — the
        // match address must be computed from the slice's own real memory
        // location, matching the original's `module + textSection->VirtualAddress + i`.
        let mut addr = module.as_ptr() as usize + offset;

        if sig.read {
            let wildcard_offset = first_wildcard_byte_offset(pattern);
            // SAFETY: `addr + wildcard_offset` points inside `module`'s backing
            // memory at the wildcard byte; the original reads a full 8-byte
            // pointer here regardless of the true immediate width, and that
            // quirk is preserved exactly (do not narrow to a 4-byte read).
            // `read_unaligned` because the wildcard offset is an arbitrary
            // byte position, not guaranteed to be 8-byte aligned (the original
            // C++ `reinterpret_cast` doesn't require alignment either).
            addr = unsafe { ((addr + wildcard_offset) as *const usize).read_unaligned() };
        } else if sig.relative {
            // SAFETY: `addr + 3` points at the 4-byte displacement field
            // within the matched pattern's bytes; read unaligned for the
            // same reason as above.
            let disp = unsafe { ((addr + 3) as *const u32).read_unaligned() };
            // Zero-extended (u32 -> usize), NOT sign-extended, matching the
            // original's `uint32_t*` cast exactly.
            addr = addr.wrapping_add(disp as usize).wrapping_add(7);
        // SAFETY: `addr` points inside `module`'s backing memory at the
        // matched pattern's first byte.
        } else if unsafe { *(addr as *const u8) } == 0xE8 {
            // SAFETY: `addr` points at the matched opcode byte within `module`.
            let disp = unsafe { ((addr + 1) as *const u32).read_unaligned() };
            // Zero-extended (u32 -> usize), NOT sign-extended, matching the
            // original's `uint32_t*` cast exactly.
            addr = addr.wrapping_add(disp as usize).wrapping_add(5);
        }

        if sig.sub_base {
            addr -= base;
        }

        addr = (addr as i64 + sig.additional as i64) as usize;
        // The original's very last step is `*sig.offset =
        // reinterpret_cast<std::uint32_t>(address)` — truncating the fully
        // resolved value down to 32 bits, unconditionally. For `sub_base:
        // true` signatures this is a no-op (the value is already a small
        // relative offset); for `read: true` signatures it's load-bearing:
        // that branch deliberately reads a full 8-byte value at a narrower
        // immediate's location, picking up trailing garbage bytes from
        // whatever instruction follows in the game's code, and this
        // truncation is what discards them.
        addr = addr as u32 as usize;
        return Some(addr);
    }
    None
}

// Minimal, field-truncated mirrors of the PE format's own `IMAGE_DOS_HEADER`
// / `IMAGE_NT_HEADERS64` / `IMAGE_FILE_HEADER` / `IMAGE_SECTION_HEADER`
// structs (declared locally instead of pulled from `windows` — this file has
// no other reason to depend on it, and the PE header layout is a stable,
// decades-old ABI). Each struct only declares the fields actually read,
// via leading padding bytes to keep every declared field at its real,
// official offset.
#[repr(C)]
struct ImageDosHeader {
    _reserved: [u8; 0x3C],
    e_lfanew: i32,
}

#[repr(C)]
struct ImageFileHeader {
    _reserved: [u8; 16],
    size_of_optional_header: u16,
    _reserved2: u16,
}

#[repr(C)]
struct ImageNtHeaders64 {
    _signature: u32,
    file_header: ImageFileHeader,
    // `OptionalHeader` itself follows here, but only its address (right
    // after `file_header`) is needed, never its contents.
}

#[repr(C)]
struct ImageSectionHeader {
    _name: [u8; 8],
    _misc: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
}

/// Walks the PE header of an already-loaded module to find its `.text`
/// section, matching the original's `IMAGE_FIRST_SECTION` macro. Restricting
/// the scan to `.text` (rather than the whole module image) keeps the same
/// false-positive-match risk profile as the original — scanning `.rdata`/
/// `.data` too could coincidentally match a short signature in data bytes.
pub unsafe fn text_section(module_base: usize) -> &'static [u8] {
    // SAFETY: caller guarantees `module_base` points at a valid, fully
    // loaded PE image's DOS header for the lifetime of the returned slice.
    let dos = unsafe { &*(module_base as *const ImageDosHeader) };
    // SAFETY: `dos.e_lfanew` is the validated-by-loader offset to the PE
    // NT headers within the same image.
    let nt = unsafe { &*((module_base as isize + dos.e_lfanew as isize) as *const ImageNtHeaders64) };
    let first_section = ((&raw const nt.file_header as usize + size_of::<ImageFileHeader>())
        + nt.file_header.size_of_optional_header as usize) as *const ImageSectionHeader;
    // SAFETY: `first_section` points at the first `IMAGE_SECTION_HEADER`
    // immediately following the optional header, per the PE format; .text
    // is the first section in this binary.
    let section = unsafe { &*first_section };
    let start = module_base + section.virtual_address as usize;
    // SAFETY: `start` is within the loaded module's `.text` section, which
    // the OS loader guarantees is fully committed and readable for
    // `section.size_of_raw_data` bytes.
    unsafe { std::slice::from_raw_parts(start as *const u8, section.size_of_raw_data as usize) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_module(prefix_len: usize, bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0x90u8; prefix_len]; // NOP padding before the pattern
        v.extend_from_slice(bytes);
        v.extend_from_slice(&[0x90; 16]); // trailing padding
        v
    }

    #[test]
    fn returns_the_first_match_not_the_last_when_several_exist() {
        // The reference `find_signature` returns the FIRST match; a callback
        // that fails to stop early would leave `found` on the LAST one, which
        // for a multi-match signature (e.g. GAME_CLIENT_SIG) resolves to the
        // wrong slot and hangs `wait_for_game_client` forever.
        let mut bytes = vec![0xAAu8, 0xBB, 0xCC];
        bytes.extend_from_slice(&[0x90; 8]);
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // a second, later match
        let module = make_module(4, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["AA BB CC"], sub_base: true, read: false, relative: false, additional: 0 };
        // First match sits at prefix offset 4 (== base+4-base after sub_base).
        assert_eq!(unsafe { resolve(base, &module, &sig) }, Some(4));
    }

    #[test]
    fn plain_call_rel32_resolves_to_call_target() {
        // E8 rel32 at offset 10: target = (addr_of_E8 + 5) + rel32.
        // rel32 = 0x20, so target = base + 10 + 5 + 0x20 = base + 0x2F.
        // `sub_base: true` matches every real call-rel32 signature here
        // (PUSH_FN_SIG/UPDATE_FN_SIG) and keeps the result small enough to
        // survive the final u32 truncation.
        let module = make_module(10, &[0xE8, 0x20, 0x00, 0x00, 0x00]);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["E8 ? ? ? ?"], sub_base: true, read: false, relative: false, additional: 0 };
        let resolved = unsafe { resolve(base, &module, &sig) };
        assert_eq!(resolved, Some(0x2F));
        drop(module); // keep it alive until here
    }

    #[test]
    fn relative_mov_resolves_with_zero_extension_not_sign_extension() {
        // "48 8B 05 ? ? ? ?" = mov reg, [rip+disp32]. A NEGATIVE-looking
        // displacement (top bit set) would go BACKWARD past the module start
        // if sign-extended; the original's uint32_t* cast zero-extends, so
        // the result must land far FORWARD. This pins that behavior down.
        let disp: u32 = 0x8000_0000; // negative if treated as i32
        let mut bytes = vec![0x48, 0x8B, 0x05];
        bytes.extend_from_slice(&disp.to_le_bytes());
        let module = make_module(10, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["48 8B 05 ? ? ? ?"], sub_base: true, read: false, relative: true, additional: 0 };
        let resolved = unsafe { resolve(base, &module, &sig) };
        let expected = 10usize.wrapping_add(disp as usize).wrapping_add(7); // zero-extended
        assert_eq!(resolved, Some(expected));
        assert!(resolved.unwrap() > 0x7FFF_FFFF, "zero-extension must move forward, not backward");
    }

    #[test]
    fn sub_base_and_additional_are_applied_after_extraction() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["AA BB CC"], sub_base: true, read: false, relative: false, additional: 5 };
        let resolved = unsafe { resolve(base, &module, &sig) };
        // match address (base+4) minus base, plus additional(5) == 9
        assert_eq!(resolved, Some(9));
    }

    #[test]
    fn read_mode_dereferences_a_pointer_sized_value_at_the_wildcard() {
        // "AA ? ? BB" — wildcard starts at char index 3, /3 == byte offset 1.
        let target: usize = 0xDEAD_BEEF;
        let mut bytes = vec![0xAAu8];
        bytes.extend_from_slice(&target.to_le_bytes());
        bytes.push(0xBB);
        let module = make_module(4, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["AA ? ? ? ? ? ? ? ? BB"], sub_base: false, read: true, relative: false, additional: 0 };
        let resolved = unsafe { resolve(base, &module, &sig) };
        assert_eq!(resolved, Some(target));
    }

    #[test]
    fn falls_through_to_the_next_pattern_when_the_first_does_not_match() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["11 22 33", "AA BB CC"], sub_base: false, read: false, relative: false, additional: 0 };
        assert!(unsafe { resolve(base, &module, &sig) }.is_some());
    }

    #[test]
    fn returns_none_when_nothing_matches() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature { patterns: &["11 22 33"], sub_base: false, read: false, relative: false, additional: 0 };
        assert!(unsafe { resolve(base, &module, &sig) }.is_none());
    }
}
