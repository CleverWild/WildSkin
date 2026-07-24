pub struct Signature {
    pub patterns: &'static [&'static str],
    pub sub_base: bool,
    pub read: bool,
    pub relative: bool,
    pub additional: i32,
}

fn find_first(haystack: &[u8], pattern: &str) -> Option<usize> {
    let builder = aobscan::PatternBuilder::from_ida_style(pattern).ok()?;
    // Single-threaded on purpose: a multi-threaded scan saturates every core,
    // and across the 14 sequential sigs at injection that starved the game and
    // froze it for seconds. Linear scan over .text is fast enough.
    let scanner = builder.with_threads(1).ok()?.build();
    let mut found = None;
    scanner.scan(haystack, |offset| {
        found = Some(offset);
        false // stop at the first match
    });
    found
}

/// Byte offset of the pattern's first wildcard token. Each token is 2 hex
/// chars + 1 space, hence `/ 3`.
#[allow(
    clippy::integer_division,
    reason = "exact division by the fixed 3-char token width, not an approximation"
)]
fn first_wildcard_byte_offset(pattern: &str) -> usize {
    pattern
        .find('?')
        .expect("read signature must contain a wildcard")
        / 3
}

pub unsafe fn resolve(base: usize, module: &[u8], sig: &Signature) -> Option<usize> {
    for pattern in sig.patterns {
        let Some(offset) = find_first(module, pattern) else {
            continue;
        };
        // `module` is the `.text` slice starting at `base + VirtualAddress`,
        // not `base`, so compute the match address from the slice's own ptr.
        let mut addr = module.as_ptr() as usize + offset;

        if sig.read {
            let wildcard_offset = first_wildcard_byte_offset(pattern);
            // SAFETY: `addr + wildcard_offset` is inside `module` at the
            // wildcard byte. Read a full 8-byte pointer regardless of the true
            // immediate width (load-bearing, do not narrow to 4 bytes);
            // `read_unaligned` since the offset isn't alignment-guaranteed.
            addr = unsafe { ((addr + wildcard_offset) as *const usize).read_unaligned() };
        } else if sig.relative {
            // SAFETY: `addr + 3` is the 4-byte displacement field; unaligned as above.
            let disp = unsafe { ((addr + 3) as *const u32).read_unaligned() };
            // Zero-extended (u32 -> usize), NOT sign-extended.
            addr = addr.wrapping_add(disp as usize).wrapping_add(7);
        // SAFETY: `addr` is inside `module` at the matched pattern's first byte.
        } else if unsafe { *(addr as *const u8) } == 0xE8 {
            // SAFETY: `addr` points at the matched opcode byte.
            let disp = unsafe { ((addr + 1) as *const u32).read_unaligned() };
            // Zero-extended (u32 -> usize), NOT sign-extended.
            addr = addr.wrapping_add(disp as usize).wrapping_add(5);
        }

        if sig.sub_base {
            addr -= base;
        }

        addr = (addr as i64 + sig.additional as i64) as usize;
        // Truncate to 32 bits unconditionally. No-op for `sub_base` (already a
        // small offset); load-bearing for `read`, which discards the trailing
        // garbage bytes the full 8-byte read picked up.
        addr = addr as u32 as usize;
        return Some(addr);
    }
    None
}

// Minimal local mirrors of the PE `IMAGE_DOS_HEADER`/`IMAGE_NT_HEADERS64`/
// `IMAGE_FILE_HEADER`/`IMAGE_SECTION_HEADER` structs (avoids a `windows` dep
// for a stable ABI). Padding keeps each declared field at its real offset.
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
    // `OptionalHeader` follows; only its address is needed, not its contents.
}

#[repr(C)]
struct ImageSectionHeader {
    _name: [u8; 8],
    _misc: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
}

/// Walks a loaded module's PE header to find its `.text` section. Restricting
/// scans to `.text` avoids false-positive matches in `.rdata`/`.data`.
pub unsafe fn text_section(module_base: usize) -> &'static [u8] {
    // SAFETY: caller guarantees `module_base` points at a valid, fully
    // loaded PE image's DOS header for the lifetime of the returned slice.
    let dos = unsafe { &*(module_base as *const ImageDosHeader) };
    // SAFETY: `dos.e_lfanew` is the validated-by-loader offset to the PE
    // NT headers within the same image.
    let nt =
        unsafe { &*((module_base as isize + dos.e_lfanew as isize) as *const ImageNtHeaders64) };
    let first_section = ((&raw const nt.file_header as usize + size_of::<ImageFileHeader>())
        + nt.file_header.size_of_optional_header as usize)
        as *const ImageSectionHeader;
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
        // Must return the FIRST match: stopping late leaves `found` on the
        // LAST, which for a multi-match sig resolves the wrong slot and hangs
        // `wait_for_game_client` forever.
        let mut bytes = vec![0xAAu8, 0xBB, 0xCC];
        bytes.extend_from_slice(&[0x90; 8]);
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // a second, later match
        let module = make_module(4, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["AA BB CC"],
            sub_base: true,
            read: false,
            relative: false,
            additional: 0,
        };
        // First match sits at prefix offset 4 (== base+4-base after sub_base).
        assert_eq!(unsafe { resolve(base, &module, &sig) }, Some(4));
    }

    #[test]
    fn plain_call_rel32_resolves_to_call_target() {
        // E8 rel32 at offset 10: target = 10 + 5 + 0x20 = 0x2F.
        // `sub_base: true` keeps the result small enough to survive u32 trunc.
        let module = make_module(10, &[0xE8, 0x20, 0x00, 0x00, 0x00]);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["E8 ? ? ? ?"],
            sub_base: true,
            read: false,
            relative: false,
            additional: 0,
        };
        let resolved = unsafe { resolve(base, &module, &sig) };
        assert_eq!(resolved, Some(0x2F));
        drop(module); // keep it alive until here
    }

    #[test]
    fn relative_mov_resolves_with_zero_extension_not_sign_extension() {
        // mov reg, [rip+disp32]. A top-bit-set disp would go backward if
        // sign-extended; zero-extension must land forward. Pins that down.
        let disp: u32 = 0x8000_0000; // negative if treated as i32
        let mut bytes = vec![0x48, 0x8B, 0x05];
        bytes.extend_from_slice(&disp.to_le_bytes());
        let module = make_module(10, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["48 8B 05 ? ? ? ?"],
            sub_base: true,
            read: false,
            relative: true,
            additional: 0,
        };
        let resolved = unsafe { resolve(base, &module, &sig) };
        let expected = 10usize.wrapping_add(disp as usize).wrapping_add(7); // zero-extended
        assert_eq!(resolved, Some(expected));
        assert!(
            resolved.unwrap() > 0x7FFF_FFFF,
            "zero-extension must move forward, not backward"
        );
    }

    #[test]
    fn sub_base_and_additional_are_applied_after_extraction() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["AA BB CC"],
            sub_base: true,
            read: false,
            relative: false,
            additional: 5,
        };
        let resolved = unsafe { resolve(base, &module, &sig) };
        // match address (base+4) minus base, plus additional(5) == 9
        assert_eq!(resolved, Some(9));
    }

    #[test]
    fn read_mode_dereferences_a_pointer_sized_value_at_the_wildcard() {
        // "AA ? ? BB": wildcard starts at char index 3, /3 == byte offset 1.
        let target: usize = 0xDEAD_BEEF;
        let mut bytes = vec![0xAAu8];
        bytes.extend_from_slice(&target.to_le_bytes());
        bytes.push(0xBB);
        let module = make_module(4, &bytes);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["AA ? ? ? ? ? ? ? ? BB"],
            sub_base: false,
            read: true,
            relative: false,
            additional: 0,
        };
        let resolved = unsafe { resolve(base, &module, &sig) };
        assert_eq!(resolved, Some(target));
    }

    #[test]
    fn falls_through_to_the_next_pattern_when_the_first_does_not_match() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["11 22 33", "AA BB CC"],
            sub_base: false,
            read: false,
            relative: false,
            additional: 0,
        };
        assert!(unsafe { resolve(base, &module, &sig) }.is_some());
    }

    #[test]
    fn returns_none_when_nothing_matches() {
        let module = make_module(4, &[0xAA, 0xBB, 0xCC]);
        let base = module.as_ptr() as usize;
        let sig = Signature {
            patterns: &["11 22 33"],
            sub_base: false,
            read: false,
            relative: false,
            additional: 0,
        };
        assert!(unsafe { resolve(base, &module, &sig) }.is_none());
    }
}
