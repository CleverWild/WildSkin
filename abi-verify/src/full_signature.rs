//! Confirms that bytes at an *already-known* position still look as
//! expected, as opposed to `resolve`'s job of finding a pattern's position
//! in the first place.
//!
//! Deliberately doesn't reuse `aobscan` (which `resolve.rs` depends on for
//! searching): `aobscan`'s single-threaded scan loop has a documented
//! off-by-one that misses the final possible match position when the
//! pattern's length equals the searched buffer's length exactly (see the
//! "off-by-one" comment on `abi-verify-macro`'s `section_bytes` test
//! helper). That's exactly the shape this module's callers tend to produce
//! (a resolved function's own tail-trimmed bytes), so matching directly at a
//! fixed start position sidesteps that bug entirely instead of working
//! around it again.

/// Parses an IDA-style AOB pattern (space-separated hex byte pairs, `?` or
/// `??` for a wildcard byte) into a sequence of optional expected bytes.
/// Returns `None` if any token isn't valid hex and isn't a wildcard.
fn parse_pattern(pattern: &str) -> Option<Vec<Option<u8>>> {
    pattern
        .split_whitespace()
        .map(|token| match token {
            "?" | "??" => Some(None),
            _ if token.len() == 2 => u8::from_str_radix(token, 16).ok().map(Some),
            _ => None,
        })
        .collect()
}

/// Byte slots `pattern` describes (wildcards included), `None` if malformed —
/// lets a mismatch report quote back exactly that many real bytes.
#[must_use]
pub fn token_count(pattern: &str) -> Option<usize> {
    parse_pattern(pattern).map(|expected| expected.len())
}

/// Checks whether `code` matches `pattern` starting at `code[0]` (wildcards
/// match any byte). Returns:
/// - `None` if `pattern` itself is malformed (see `parse_pattern`)
/// - `Some(false)` if `pattern` is well-formed but `code` is shorter than it,
///   or the bytes present don't match
/// - `Some(true)` if every non-wildcard byte matches
#[must_use]
pub fn matches_at_start(code: &[u8], pattern: &str) -> Option<bool> {
    let expected = parse_pattern(pattern)?;
    if code.len() < expected.len() {
        return Some(false);
    }
    Some(expected.iter().zip(code).all(|(&want, &got)| want.is_none_or(|w| w == got)))
}

#[cfg(test)]
mod tests {
    use super::matches_at_start;

    #[test]
    fn exact_match_no_wildcards() {
        assert_eq!(matches_at_start(&[0xAA, 0xBB, 0xCC], "AA BB CC"), Some(true));
    }

    #[test]
    fn wildcards_in_the_middle_match_any_byte() {
        assert_eq!(matches_at_start(&[0xAA, 0x00, 0xCC], "AA ? CC"), Some(true));
        assert_eq!(matches_at_start(&[0xAA, 0xFF, 0xCC], "AA ?? CC"), Some(true));
    }

    #[test]
    fn one_byte_mismatch_fails() {
        assert_eq!(matches_at_start(&[0xAA, 0xBB, 0xCC], "AA BB CD"), Some(false));
    }

    #[test]
    fn code_shorter_than_pattern_is_false_not_none() {
        assert_eq!(matches_at_start(&[0xAA, 0xBB], "AA BB CC"), Some(false));
    }

    #[test]
    fn malformed_pattern_invalid_hex_token() {
        assert_eq!(matches_at_start(&[0xAA], "ZZ"), None);
    }

    #[test]
    fn malformed_pattern_wrong_token_length() {
        assert_eq!(matches_at_start(&[0xAA], "A"), None);
        assert_eq!(matches_at_start(&[0xAA, 0xBB], "AAA"), None);
    }

    #[test]
    fn empty_pattern_matches_trivially() {
        // A pattern with zero expected bytes is vacuously satisfied by any
        // `code`, including empty `code` — pinned down explicitly since
        // it's an edge case worth being deliberate about.
        assert_eq!(matches_at_start(&[], ""), Some(true));
        assert_eq!(matches_at_start(&[0xAA, 0xBB], ""), Some(true));
    }

    #[test]
    fn code_longer_than_pattern_still_matches_at_start() {
        assert_eq!(matches_at_start(&[0xAA, 0xBB, 0xCC, 0xDD], "AA BB"), Some(true));
    }
}
