pub const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV_PRIME: u64 = 1_099_511_628_211;

pub const fn fnv1a(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    // Hash the NUL terminator too (one extra mix round on '\0').
    hash ^= 0u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_at_compile_time_and_runtime() {
        const COMPILE_TIME: u64 = fnv1a("Katarina");
        let owned = String::from("Katarina");
        assert_eq!(COMPILE_TIME, fnv1a(&owned));
    }

    #[test]
    fn different_strings_differ() {
        assert_ne!(fnv1a("Katarina"), fnv1a("Renekton"));
    }

    #[test]
    fn hashes_the_nul_terminator_like_the_original() {
        // The NUL-terminator step is one extra mix round beyond the visible
        // bytes; XOR-by-zero is a no-op, so it's just one more multiply.
        let mut expected = FNV_OFFSET_BASIS;
        expected ^= b'A' as u64;
        expected = expected.wrapping_mul(FNV_PRIME);
        expected = expected.wrapping_mul(FNV_PRIME); // the NUL-terminator round
        assert_eq!(fnv1a("A"), expected);
    }
}
