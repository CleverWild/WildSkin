//! Resolves an AOB signature to a byte offset.
//!
//! Mirrors `WildSkin-rs`'s runtime `memory::scanner::resolve`, but for use at
//! compile time against an on-disk copy of the target executable.

/// Why a signature failed to resolve to exactly one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    /// The pattern matched nowhere in the searched bytes.
    NotFound,
    /// Pattern resolved to two or more distinct targets, so it's no longer a
    /// unique locator. Count is distinct targets found before stopping (>= 2).
    Ambiguous(usize),
}

/// Scans `text` for `pattern`, mapping each match to a target via `resolve_one`
/// (`None` rejects a spurious match), collecting distinct targets. Stops at the
/// second distinct target so an ambiguous pattern bails early.
fn resolve_unique(
    text: &[u8],
    pattern: &str,
    resolve_one: impl Fn(usize) -> Option<usize> + Send + Sync,
) -> Result<usize, ResolveError> {
    let Some(scanner) = aobscan::PatternBuilder::from_ida_style(pattern)
        .ok()
        .and_then(|builder| builder.with_threads(1).ok())
        .map(aobscan::PatternBuilder::build)
    else {
        // A malformed pattern can't match anything; treat as not found.
        return Err(ResolveError::NotFound);
    };

    let mut targets: Vec<usize> = Vec::new();
    scanner.scan(text, |offset| {
        if let Some(target) = resolve_one(offset)
            && !targets.contains(&target)
        {
            targets.push(target);
        }
        // aobscan callback: `true` continues, `false` stops. Stop once a
        // second distinct target proves ambiguity.
        targets.len() < 2
    });

    match targets.as_slice() {
        [] => Err(ResolveError::NotFound),
        [single] => Ok(*single),
        _ => Err(ResolveError::Ambiguous(targets.len())),
    }
}

/// Resolves a signature whose match lands on the target's own first byte
/// (no indirection), e.g. a prologue pattern.
pub fn resolve_direct(text: &[u8], pattern: &str) -> Result<usize, ResolveError> {
    resolve_unique(text, pattern, Some)
}

/// Resolves a signature whose match lands on an `E8 rel32` CALL, following it
/// to the target. 
/// 
/// Mirrors `memory::scanner::resolve`'s `sub_base` handling
/// (identify a function via a call site). Two sites to the same function are
/// one distinct target, not ambiguous.
pub fn resolve_call_target(text: &[u8], pattern: &str) -> Result<usize, ResolveError> {
    resolve_unique(text, pattern, |match_offset| {
        if text.get(match_offset).copied() != Some(0xE8) {
            return None;
        }
        let rel_bytes = text.get(match_offset + 1..match_offset + 5)?;
        let rel = i32::from_le_bytes(rel_bytes.try_into().ok()?);
        let target = i64::try_from(match_offset).ok()? + 5 + i64::from(rel);
        usize::try_from(target).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::{ResolveError, resolve_call_target, resolve_direct};

    #[test]
    fn resolve_direct_finds_the_match_position() {
        let text = [0x90, 0x90, 0xAA, 0xBB, 0xCC, 0x90];
        assert_eq!(resolve_direct(&text, "AA BB CC"), Ok(2));
    }

    #[test]
    fn resolve_direct_returns_not_found_when_absent() {
        let text = [0x90, 0x90, 0x90];
        assert_eq!(
            resolve_direct(&text, "AA BB CC"),
            Err(ResolveError::NotFound)
        );
    }

    #[test]
    fn resolve_direct_is_ambiguous_on_two_matches() {
        // "AA BB" occurs at offsets 0 and 3 -> two distinct targets.
        let text = [0xAA, 0xBB, 0x90, 0xAA, 0xBB, 0x90, 0x90];
        assert_eq!(
            resolve_direct(&text, "AA BB"),
            Err(ResolveError::Ambiguous(2))
        );
    }

    #[test]
    fn resolve_call_target_follows_the_call() {
        // E8 rel32 at offset 2; target = (2 + 5) + rel32.
        // rel32 = 0x10, so target = 7 + 0x10 = 0x17.
        let mut text = vec![0x90, 0x90, 0xE8, 0x10, 0x00, 0x00, 0x00];
        text.resize(0x17 + 1, 0x90);
        text[0x17] = 0xCC; // marker at the resolved target, not required by the fn, just for sanity
        assert_eq!(resolve_call_target(&text, "E8 ? ? ? ?"), Ok(0x17));
    }

    #[test]
    fn resolve_call_target_returns_not_found_when_match_is_not_a_call() {
        let text = [0x90, 0x90, 0xAA, 0xBB, 0xCC, 0x90];
        assert_eq!(
            resolve_call_target(&text, "AA BB CC"),
            Err(ResolveError::NotFound)
        );
    }

    #[test]
    fn resolve_call_target_two_sites_to_the_same_function_are_not_ambiguous() {
        // Two `E8` call sites (offsets 0 and 5) whose rel32s both point at the
        // same absolute target (0x20) -> one distinct target, not ambiguous.
        let mut text = vec![0u8; 0x21];
        // call at 0: target 0x20 => rel32 = 0x20 - (0 + 5) = 0x1B
        text[0] = 0xE8;
        text[1..5].copy_from_slice(&0x1Bi32.to_le_bytes());
        // call at 5: target 0x20 => rel32 = 0x20 - (5 + 5) = 0x16
        text[5] = 0xE8;
        text[6..10].copy_from_slice(&0x16i32.to_le_bytes());
        assert_eq!(resolve_call_target(&text, "E8 ? ? ? ?"), Ok(0x20));
    }
}
