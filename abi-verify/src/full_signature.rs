//! Confirms bytes at an already-known position still look as expected, unlike
//! `resolve`'s job of finding a pattern's position.
//!
//! Deliberately doesn't reuse `aobscan`: its scan loop has an off-by-one that
//! misses the final match position when pattern length equals buffer length
//! (see the `section_bytes` test helper in `abi-verify-macro`). This module's
//! callers hit exactly that shape, so matching at a fixed start sidesteps it.

/// Parses an IDA-style AOB pattern (hex byte pairs, `?`/`??` = wildcard) into
/// optional expected bytes. `None` if a token isn't hex and isn't a wildcard.
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

/// Byte slots `pattern` describes (wildcards included), `None` if malformed;
/// lets a mismatch quote back exactly that many real bytes.
#[must_use]
pub fn token_count(pattern: &str) -> Option<usize> {
    parse_pattern(pattern).map(|expected| expected.len())
}

/// Whether `code` matches `pattern` at `code[0]` (wildcards match any byte):
/// - `None` if `pattern` is malformed (see `parse_pattern`)
/// - `Some(false)` if `code` is shorter than it, or bytes don't match
/// - `Some(true)` if every non-wildcard byte matches
#[must_use]
pub fn matches_at_start(code: &[u8], pattern: &str) -> Option<bool> {
    let expected = parse_pattern(pattern)?;
    if code.len() < expected.len() {
        return Some(false);
    }
    Some(
        expected
            .iter()
            .zip(code)
            .all(|(&want, &got)| want.is_none_or(|w| w == got)),
    )
}

#[cfg(test)]
mod tests {
    use super::matches_at_start;

    #[test]
    fn exact_match_no_wildcards() {
        assert_eq!(
            matches_at_start(&[0xAA, 0xBB, 0xCC], "AA BB CC"),
            Some(true)
        );
    }

    #[test]
    fn wildcards_in_the_middle_match_any_byte() {
        assert_eq!(matches_at_start(&[0xAA, 0x00, 0xCC], "AA ? CC"), Some(true));
        assert_eq!(
            matches_at_start(&[0xAA, 0xFF, 0xCC], "AA ?? CC"),
            Some(true)
        );
    }

    #[test]
    fn one_byte_mismatch_fails() {
        assert_eq!(
            matches_at_start(&[0xAA, 0xBB, 0xCC], "AA BB CD"),
            Some(false)
        );
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
        // Zero expected bytes is vacuously satisfied by any `code`, empty
        // included; pinned as a deliberate edge case.
        assert_eq!(matches_at_start(&[], ""), Some(true));
        assert_eq!(matches_at_start(&[0xAA, 0xBB], ""), Some(true));
    }

    #[test]
    fn code_longer_than_pattern_still_matches_at_start() {
        assert_eq!(
            matches_at_start(&[0xAA, 0xBB, 0xCC, 0xDD], "AA BB"),
            Some(true)
        );
    }
}
