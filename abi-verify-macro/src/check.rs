//! Plain, unit-testable core logic behind `#[verify_abi]`.
//!
//! Deliberately has nothing to do with proc-macro machinery (no `syn` /
//! `proc_macro2` types anywhere in this file) so it can be exercised with
//! ordinary `#[test]` functions instead of heavyweight tools like `trybuild`.
//!
//! Note on the `exe_path` parameter: the brief this was built from originally
//! had `check_abi` read the env var itself (`exe_path_env_var: &str`). That
//! was changed to accept the already-resolved path as `Option<&Path>`
//! instead, with the proc-macro wrapper in `lib.rs` doing the
//! `std::env::var` lookup. Reading env vars from within unit tests is
//! flaky (env vars are process-global, so parallel tests mutating them can
//! race each other); resolving the path in the wrapper and passing it down
//! removes that mutation from tests entirely, per option (c) in the brief.

use abi_verify::arg_count::RecoveredArgCount;
use abi_verify::resolve::ResolveError;
use abi_verify::{DISABLED_ENV_VAR, EXE_PATH_ENV_VAR};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One-entry `.text` cache: without it every annotated item re-reads the same
/// ~27 MB section. One entry is enough — every item in a compilation resolves
/// the same path.
static TEXT_CACHE: Mutex<Option<(PathBuf, Arc<[u8]>)>> = Mutex::new(None);

fn text_section(exe_path: &Path) -> Result<Arc<[u8]>, String> {
    let hit = TEXT_CACHE
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref().filter(|(path, _)| path == exe_path).map(|(_, text)| Arc::clone(text)));
    if let Some(text) = hit {
        return Ok(text);
    }

    let text: Arc<[u8]> = abi_verify::pe::read_text_section(exe_path)
        .map_err(|error| format!("abi-verify: failed to read .text section from {}: {error}", exe_path.display()))?
        .into();
    if let Ok(mut cache) = TEXT_CACHE.lock() {
        *cache = Some((exe_path.to_path_buf(), Arc::clone(&text)));
    }
    Ok(text)
}

/// Resolves the annotated item's locator pattern to a single function offset
/// within `text`, turning [`ResolveError`] into a human-readable message.
fn resolve_offset(args: &AbiCheckArgs<'_>, text: &[u8]) -> Result<usize, String> {
    let resolved = if args.call_target {
        abi_verify::resolve::resolve_call_target(text, args.pattern)
    } else {
        abi_verify::resolve::resolve_direct(text, args.pattern)
    };
    resolved.map_err(|error| match error {
        ResolveError::NotFound => format!(
            "abi-verify: could not locate `{}` in the game binary: its AOB pattern matched nothing in .text.\n  \
             pattern: {}\n  \
             A game patch most likely moved or rewrote the function: re-find it via decompiler and update \
             `pattern` to bytes that are stable across builds.",
            args.item_name, args.pattern
        ),
        ResolveError::Ambiguous(count) => format!(
            "abi-verify: the AOB pattern for `{}` is no longer a unique locator: it matches {count} distinct \
             functions, so the wrong one could be verified.\n  \
             pattern: {}\n  \
             Lengthen it (append more of the function's opcode bytes) until exactly one function matches.",
            args.item_name, args.pattern
        ),
    })
}

/// Resolves the exe path to its `.text` section. `Ok(None)` = no exe
/// configured, so every check is skipped; `Err` = a path was given but isn't
/// usable, which is loud (setting the var at all implies meaning it).
fn resolve_text(exe_path: Option<&Path>) -> Result<Option<Arc<[u8]>>, String> {
    let Some(exe_path) = exe_path else {
        return Ok(None);
    };
    if std::fs::metadata(exe_path).is_err() {
        return Err(format!(
            "abi-verify: configured game executable path does not exist: {}\n  \
             `{EXE_PATH_ENV_VAR}` points somewhere that isn't there. Fix it, unset it to fall back to \
             auto-detecting a standard install, or set `{DISABLED_ENV_VAR}=true` to skip these checks entirely.",
            exe_path.display()
        ));
    }
    text_section(exe_path).map(Some)
}

/// Runs every exe-dependent check for one annotated item, returning one
/// message per failure so a single failing check can't mask another.
///
/// The `.text` read, the pattern resolution and the disassembly happen once
/// and feed all three comparisons, which are themselves pure functions over
/// the resolved bytes (and unit-tested as such). A failure to get that far —
/// no exe, unreadable exe, unresolvable pattern — is a single message, since
/// none of the comparisons can run at all.
pub fn run_checks(
    args: &AbiCheckArgs<'_>,
    exe_path: Option<&Path>,
    declared_widths: &[Option<u8>],
    param_names: &[Option<String>],
) -> Vec<String> {
    let text = match resolve_text(exe_path) {
        Ok(Some(text)) => text,
        Ok(None) => return Vec::new(),
        Err(error) => return vec![error],
    };
    let offset = match resolve_offset(args, &text) {
        Ok(offset) => offset,
        Err(error) => return vec![error],
    };

    let body = &text[offset..];
    let recovered = abi_verify::arg_count::recover_arg_count(body);
    [
        compare_arg_count(args, &recovered),
        compare_full_signature(args, body),
        compare_widths(declared_widths, &recovered, param_names, args.item_name),
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect()
}

pub struct AbiCheckArgs<'a> {
    pub pattern: &'a str,
    pub call_target: bool,
    /// Parameter count read off the annotated type itself. `None` when the item
    /// isn't a bare-fn type, in which case there is nothing to compare.
    pub declared_args: Option<u8>,
    pub item_name: &'a str,
    /// Developer-recorded byte template for [`check_full_signature`]. `None`
    /// means that check isn't requested for this item at all (fully
    /// optional, backward compatible with items that only want the
    /// arg-count check).
    pub full_signature: Option<&'a str>,
}

/// Compares a single FFI function pointer's declared parameter count against
/// what disassembling the real game function recovered. Pure: no filesystem,
/// no pattern resolution.
fn compare_arg_count(args: &AbiCheckArgs<'_>, recovered: &RecoveredArgCount) -> Result<(), String> {
    let Some(declared) = args.declared_args else {
        return Ok(());
    };
    let total = recovered.total();
    if total != declared {
        return Err(format!(
            "abi-verify: ABI drift in `{}` — the type declares {declared} parameter(s), the binary uses {total} \
             ({} in registers + {} on the stack).\n  \
             recovered stack-arg widths, in order: {}\n  \
             Line those widths up against the declared parameters after the first four (which go in registers): a \
             single extra or missing entry pinpoints the added/removed slot.",
            args.item_name,
            recovered.register_args,
            recovered.stack_args,
            format_widths(&recovered.stack_arg_widths),
        ));
    }

    Ok(())
}

/// Renders widths as `[8, 4, 1, ?]`, `?` for a slot never seen read.
fn format_widths(widths: &[Option<u8>]) -> String {
    let rendered: Vec<String> =
        widths.iter().map(|width| width.map_or_else(|| "?".to_owned(), |width| width.to_string())).collect();
    format!("[{}]", rendered.join(", "))
}

/// Renders bytes as an IDA-style hex string, ready to paste into a pattern.
fn format_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ")
}

/// Compares the resolved function's actual bytes (`body`, starting at its
/// first byte) against a developer-recorded "full signature" byte template —
/// independent of the parameter-count check, and pure like it.
///
/// `args.full_signature` being `None` means this check wasn't requested for
/// this item at all: rather than making the caller branch on that, this
/// function itself treats it as a trivial `Ok(())` no-op.
fn compare_full_signature(args: &AbiCheckArgs<'_>, body: &[u8]) -> Result<(), String> {
    let Some(full_signature) = args.full_signature else {
        return Ok(());
    };

    match abi_verify::full_signature::matches_at_start(body, full_signature) {
        None => Err(format!(
            "abi-verify: the `full_signature` recorded for `{}` is malformed — every token must be a two-digit \
             hex byte or `?`.\n  \
             got: {full_signature}",
            args.item_name
        )),
        Some(false) => {
            // As many real bytes as the template describes = the corrected template.
            let actual = abi_verify::full_signature::token_count(full_signature)
                .map(|count| format_bytes(&body[..count.min(body.len())]))
                .unwrap_or_default();
            Err(format!(
                "abi-verify: `{}`'s bytes changed — the function at the resolved address no longer matches its \
                 recorded `full_signature`. This is independent of the parameter count and catches edits inside \
                 the body, or a `pattern` that drifted onto a different function.\n  \
                 recorded: {full_signature}\n  \
                 actual:   {actual}\n  \
                 If the function is still the right one, replace `full_signature` with the `actual` bytes above, \
                 re-wildcarding (`?`) any rel32 displacement — those move with every build and must not be pinned.",
                args.item_name
            ))
        }
        Some(true) => Ok(()),
    }
}

/// Checks each declared STACK-passed parameter's byte width against what the
/// disassembler recovered reading that slot in the real game binary — a third
/// check, catching a class the parameter *count* can't (e.g. a pointer arg
/// silently becoming a non-pointer, or vice versa, without the slot count
/// changing).
///
/// `declared_widths` and `param_names` are indexed by declared parameter
/// position (`None` width = a Rust type the caller couldn't map to a byte
/// size; `None` name = an unnamed parameter).
///
/// Only STACK-passed args (declared position >= 4, under the Microsoft x64
/// convention where the first four integer/pointer args go in registers) are
/// checked, and only against the pointer/8-byte boundary — the empirically
/// reliable signal (validated against the real `CharacterDataStack::Push`,
/// whose 14 stack slots' recovered widths matched their declared types
/// exactly). Register-arg widths are deliberately NOT checked: MSVC prologues
/// spill all four register args to shadow space as full 64-bit stores
/// regardless of real width, so their recovered width is meaningless. Finer
/// distinctions (e.g. `bool` vs `i32`, both <8 bytes) are also not enforced,
/// as a compiler may legitimately read a narrow value with a wider load;
/// only the 8-vs-not-8 (pointer) boundary is treated as authoritative.
///
/// Assumes every parameter is integer/pointer class (no floats/XMM, no
/// by-value aggregates) — true for this project's FFI signatures. If the
/// disassembler detected a float/vector argument (`has_float_args`), that
/// assumption is violated (a float arg consumes a shared slot index and
/// shifts the integer-position-to-stack-slot mapping), so the check is
/// skipped entirely rather than risk a false mismatch.
fn compare_widths(
    declared_widths: &[Option<u8>],
    recovered: &RecoveredArgCount,
    param_names: &[Option<String>],
    item_name: &str,
) -> Result<(), String> {
    if recovered.has_float_args {
        return Ok(());
    }
    let mut mismatches = Vec::new();
    for (idx, declared) in declared_widths.iter().enumerate() {
        // Register args (first four) — shadow-space homogenization, skip.
        if idx < 4 {
            continue;
        }
        let Some(declared) = *declared else { continue }; // unmappable Rust type
        let stack_slot = idx - 4;
        let Some(Some(recovered_width)) = recovered.stack_arg_widths.get(stack_slot).copied() else {
            continue; // slot beyond what was recovered, or never actually read
        };
        if (declared == 8) != (recovered_width == 8) {
            let name = param_names.get(idx).and_then(Option::as_deref).unwrap_or("_");
            mismatches.push(format!(
                "param #{idx} `{name}` declared {declared}-byte but the function reads that stack slot at \
                 {recovered_width} bytes"
            ));
        }
    }
    if mismatches.is_empty() {
        return Ok(());
    }
    Err(format!(
        "abi-verify: stack-argument width drift for `{item_name}` at the pointer (8-byte) boundary: {}. \
         The game may have been patched — verify the declared parameter types against the current binary. \
         (Register-argument widths are intentionally not checked: shadow-space spills make them unreliable.)",
        mismatches.join("; ")
    ))
}

#[cfg(test)]
mod width_tests {
    use super::compare_widths;
    use abi_verify::arg_count::RecoveredArgCount;

    fn recovered(stack_widths: Vec<Option<u8>>) -> RecoveredArgCount {
        RecoveredArgCount {
            register_args: 4,
            stack_args: u8::try_from(stack_widths.len()).unwrap(),
            register_arg_widths: [Some(8); 4],
            stack_arg_widths: stack_widths,
            has_float_args: false,
        }
    }

    #[test]
    fn matching_stack_widths_pass() {
        // 4 register params (widths irrelevant) + 3 stack params: ptr, i32, ptr.
        let declared = [Some(8), Some(8), Some(4), Some(4), Some(8), Some(4), Some(8)];
        let names = [None, None, None, None, Some("a".to_owned()), Some("b".to_owned()), Some("c".to_owned())];
        let rec = recovered(vec![Some(8), Some(4), Some(8)]);
        assert_eq!(compare_widths(&declared, &rec, &names, "Foo"), Ok(()));
    }

    #[test]
    fn declared_pointer_read_as_4_bytes_fails() {
        // Stack param #5 declared as an 8-byte pointer, but the function reads
        // that slot at 4 bytes -> the exact crash-class drift.
        let declared = [Some(8), Some(8), Some(4), Some(4), Some(8)];
        let names = [None, None, None, None, Some("model_ptr".to_owned())];
        let rec = recovered(vec![Some(4)]);
        let err = compare_widths(&declared, &rec, &names, "Foo").expect_err("should fail");
        assert!(err.contains("model_ptr"), "names the param: {err}");
        assert!(err.contains("param #4"), "names the position: {err}");
        assert!(err.contains("8-byte") && err.contains("4 bytes"), "names both widths: {err}");
    }

    #[test]
    fn register_arg_width_mismatch_is_not_enforced() {
        // Register params (idx 0..4) have deliberately "wrong" declared widths
        // vs. the recovered register widths — must NOT fire (registers skipped).
        let declared = [Some(1), Some(1), Some(1), Some(1)];
        let rec = recovered(vec![]);
        assert_eq!(compare_widths(&declared, &rec, &[], "Foo"), Ok(()));
    }

    #[test]
    fn float_args_skip_the_width_check_entirely() {
        // A declared 8-byte pointer stack arg read at 4 bytes would normally
        // fire — but with has_float_args set, the position mapping is
        // unreliable, so the whole check is skipped (returns Ok).
        let declared = [Some(8), Some(8), Some(4), Some(4), Some(8)];
        let mut rec = recovered(vec![Some(4)]);
        rec.has_float_args = true;
        assert_eq!(compare_widths(&declared, &rec, &[None, None, None, None, None], "Foo"), Ok(()));
    }

    #[test]
    fn non_pointer_boundary_differences_below_8_are_not_enforced() {
        // Declared bool (1) but read as 4 bytes (movzx-into-32-bit) — both
        // are non-pointer (<8), so this deliberately does NOT fire.
        let declared = [Some(8), Some(8), Some(4), Some(4), Some(1)];
        let rec = recovered(vec![Some(4)]);
        let names: [Option<String>; 5] = [None, None, None, None, None];
        assert_eq!(compare_widths(&declared, &rec, &names, "Foo"), Ok(()));
    }
}

/// Formats a compact summary of a bare-fn type's declared parameter names,
/// for appending to an error message from [`check_abi`] or
/// [`check_full_signature`] as extra context — e.g. `"this, model, _, _,
/// gear"`, with `_` standing in for a position that has no name.
///
/// Returns `None` when there's nothing worth reporting: either `names` is
/// empty (no parameters at all), or every position is unnamed (an
/// all-underscore summary would be pure noise) — in both cases, callers
/// should leave their original message untouched rather than appending it.
pub fn summarize_param_names(names: &[Option<String>]) -> Option<String> {
    if names.iter().all(Option::is_none) {
        return None;
    }
    Some(names.iter().map(|name| name.as_deref().unwrap_or("_")).collect::<Vec<_>>().join(", "))
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, run_checks, summarize_param_names, AbiCheckArgs};
    use iced_x86::{Code, Encoder, Instruction, MemoryOperand, Register};
    use std::path::Path;

    /// `run_checks` with no declared widths/names — the width check needs both
    /// and is covered on its own in `width_tests`.
    fn errors(args: &AbiCheckArgs<'_>, exe_path: Option<&Path>) -> Vec<String> {
        run_checks(args, exe_path, &[], &[])
    }

    /// The single expected failure message, or a panic naming what came instead.
    fn only_error(args: &AbiCheckArgs<'_>, exe_path: Option<&Path>) -> String {
        let mut errors = errors(args, exe_path);
        assert_eq!(errors.len(), 1, "expected exactly one failure, got {errors:?}");
        errors.remove(0)
    }

    /// Builds a minimal but structurally valid synthetic PE buffer with a
    /// single `.text` section whose raw data is `section_data`. Mirrors
    /// `abi_verify::pe`'s own `make_synthetic_pe` test helper (that one is
    /// private to its crate, so it's not reusable from here directly).
    fn make_synthetic_pe(section_data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(b"MZ");
        buf.resize(0x3C, 0);
        let nt_offset = buf.len() as i32 + 4;
        buf.extend_from_slice(&nt_offset.to_le_bytes());

        buf.extend_from_slice(b"PE\0\0");
        let size_of_optional_header: u16 = 0xF0;
        buf.extend_from_slice(&0u16.to_le_bytes()); // Machine
        buf.extend_from_slice(&1u16.to_le_bytes()); // NumberOfSections = 1
        buf.extend_from_slice(&0u32.to_le_bytes()); // TimeDateStamp
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToSymbolTable
        buf.extend_from_slice(&0u32.to_le_bytes()); // NumberOfSymbols
        buf.extend_from_slice(&size_of_optional_header.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // Characteristics

        buf.resize(buf.len() + size_of_optional_header as usize, 0);

        let pointer_to_raw_data = (buf.len() + IMAGE_SECTION_HEADER_SIZE + 4) as u32;
        buf.extend_from_slice(b".text\0\0\0");
        buf.extend_from_slice(&0u32.to_le_bytes()); // Misc/union (unused)
        buf.extend_from_slice(&0u32.to_le_bytes()); // VirtualAddress (unused here)
        buf.extend_from_slice(&(section_data.len() as u32).to_le_bytes()); // SizeOfRawData
        buf.extend_from_slice(&pointer_to_raw_data.to_le_bytes()); // PointerToRawData
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToRelocations
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToLinenumbers
        buf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfRelocations
        buf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfLinenumbers
        buf.extend_from_slice(&0u32.to_le_bytes()); // Characteristics

        buf.resize(pointer_to_raw_data as usize, 0);
        buf.extend_from_slice(section_data);

        buf
    }

    const IMAGE_SECTION_HEADER_SIZE: usize = 40;

    /// Writes `bytes` to a unique temp file and returns its path; the caller
    /// is responsible for removing it.
    fn write_temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "abi-verify-macro-check-test-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// Encodes `mov [rsp-8], rcx; mov [rsp-16], rdx; ret` — a tiny function
    /// that reads exactly 2 register parameters (RCX, RDX), per the same
    /// technique `abi-verify`'s own `arg_count` tests use.
    fn two_arg_function_bytes() -> Vec<u8> {
        let spill = |disp: i64, reg: Register| {
            Instruction::with2(Code::Mov_rm64_r64, MemoryOperand::with_base_displ(Register::RSP, disp), reg)
                .expect("mov [rsp+disp], reg64 must encode")
        };
        let instructions =
            [spill(-8, Register::RCX), spill(-16, Register::RDX), Instruction::with(Code::Retnq)];
        let mut encoder = Encoder::new(64);
        let mut rip = 0u64;
        for instruction in &instructions {
            let len = encoder.encode(instruction, rip).expect("test instruction must encode");
            rip += len as u64;
        }
        encoder.take_buffer()
    }

    /// Appends trailing NOP padding after `function_bytes`.
    ///
    /// `aobscan`'s single-threaded scan loop has an off-by-one: it never
    /// checks the final possible match position when the pattern's length
    /// equals the searched buffer's length exactly (`length = data.len() -
    /// signature.len()` then `for i in 0..length` — zero iterations when
    /// they're equal). A `.text` section that's *exactly* the function under
    /// test hits that edge case, so the padding here keeps the buffer
    /// strictly longer than the pattern being searched for.
    fn section_bytes(function_bytes: &[u8]) -> Vec<u8> {
        let mut bytes = function_bytes.to_vec();
        bytes.extend_from_slice(&[0x90; 16]);
        bytes
    }

    #[test]
    fn env_var_unset_skips_silently() {
        let args = AbiCheckArgs {
            pattern: "AA BB CC",
            call_target: false,
            declared_args: Some(99),
            item_name: "Whatever",
            full_signature: None,
        };
        assert_eq!(errors(&args, None), Vec::<String>::new());
    }

    #[test]
    fn matching_expected_args_succeeds() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("match-ok", &pe);

        let pattern = format_bytes(&function_bytes);
        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: None,
        };
        let result = errors(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn mismatched_expected_args_fails_loudly() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("mismatch", &pe);

        let pattern = format_bytes(&function_bytes);
        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(5),
            item_name: "TestFn",
            full_signature: None,
        };
        let err = only_error(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert!(err.contains("declares 5"), "message should mention declared count: {err}");
        assert!(err.contains("uses 2"), "message should mention recovered count: {err}");
        // The widths are the actionable part: they locate the added/removed slot.
        assert!(err.contains("recovered stack-arg widths"), "message should list recovered widths: {err}");
    }

    #[test]
    fn pattern_not_found_fails_loudly() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&function_bytes);
        let path = write_temp_file("no-match", &pe);

        let args = AbiCheckArgs {
            pattern: "FF FF FF FF FF FF FF FF FF FF",
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: None,
        };
        let err = only_error(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert!(err.contains("could not locate"), "message should say the function wasn't located: {err}");
        assert!(err.contains("FF FF FF"), "message should quote the pattern that failed: {err}");
    }

    #[test]
    fn nonexistent_exe_path_fails_loudly() {
        let path = std::env::temp_dir().join("abi-verify-macro-this-file-does-not-exist.exe");
        let args = AbiCheckArgs {
            pattern: "AA BB CC",
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: Some("AA BB CC"),
        };
        // One message, not one per check: nothing can be verified at all.
        let err = only_error(&args, Some(&path));
        assert!(err.contains(&path.display().to_string()), "message should mention the path: {err}");
    }

    #[test]
    fn matching_full_signature_succeeds() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("full-sig-match", &pe);

        let pattern = format_bytes(&function_bytes);
        let full_signature = format_bytes(&function_bytes);
        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: Some(&full_signature),
        };
        let result = errors(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn mismatching_full_signature_fails_loudly() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("full-sig-mismatch", &pe);

        let pattern = format_bytes(&function_bytes);
        let real_bytes_hex = format_bytes(&function_bytes);
        let mut mismatched_bytes = function_bytes;
        mismatched_bytes[0] ^= 0xFF; // flip a byte vs. the real function's bytes
        let full_signature = format_bytes(&mismatched_bytes);

        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: Some(&full_signature),
        };
        let err = only_error(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert!(err.contains("bytes changed"), "message should say the bytes changed: {err}");
        assert!(err.contains(&full_signature), "message should quote the recorded template: {err}");
        // The point of the message: the fix is pasteable straight out of it.
        assert!(err.contains(&real_bytes_hex), "message should quote the actual bytes: {err}");
    }

    #[test]
    fn malformed_full_signature_fails_loudly() {
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("full-sig-malformed", &pe);

        let pattern = format_bytes(&function_bytes);
        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(2),
            item_name: "TestFn",
            full_signature: Some("ZZ"),
        };
        let err = only_error(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert!(err.contains("malformed"), "message should mention 'malformed': {err}");
    }

    #[test]
    fn every_failing_check_reports_separately() {
        // Wrong arity AND a wrong byte template: two independent messages, so
        // neither can mask the other.
        let function_bytes = two_arg_function_bytes();
        let pe = make_synthetic_pe(&section_bytes(&function_bytes));
        let path = write_temp_file("two-failures", &pe);

        let pattern = format_bytes(&function_bytes);
        let mut mismatched_bytes = function_bytes;
        mismatched_bytes[0] ^= 0xFF;
        let full_signature = format_bytes(&mismatched_bytes);

        let args = AbiCheckArgs {
            pattern: &pattern,
            call_target: false,
            declared_args: Some(5),
            item_name: "TestFn",
            full_signature: Some(&full_signature),
        };
        let result = errors(&args, Some(&path));
        let _ = std::fs::remove_file(&path);

        assert_eq!(result.len(), 2, "expected both failures: {result:?}");
    }

    #[test]
    fn summarize_param_names_all_present() {
        let names = vec![Some("this".to_owned()), Some("model".to_owned())];
        assert_eq!(summarize_param_names(&names), Some("this, model".to_owned()));
    }

    #[test]
    fn summarize_param_names_all_none() {
        let names = vec![None, None, None];
        assert_eq!(summarize_param_names(&names), None);
    }

    #[test]
    fn summarize_param_names_mixed() {
        let names = vec![Some("this".to_owned()), None, Some("model".to_owned()), None];
        assert_eq!(summarize_param_names(&names), Some("this, _, model, _".to_owned()));
    }

    #[test]
    fn summarize_param_names_empty() {
        let names: Vec<Option<String>> = Vec::new();
        assert_eq!(summarize_param_names(&names), None);
    }
}
