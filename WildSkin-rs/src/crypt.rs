pub fn derive_key() -> u32 {
    // Matches the original: xor_key's bytes are the low bytes of one
    // __rdtsc() reading (sizeof(int32_t)=4 never needs a second reading).
    // SAFETY: `_rdtsc` is safe to call unconditionally on any x86_64 CPU;
    // it just reads the timestamp counter register.
    unsafe { core::arch::x86_64::_rdtsc() as u32 }
}

pub const fn xor_mix(value: i32, key: u32) -> i32 {
    ((value as u32) ^ !key) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_is_its_own_inverse() {
        let key = 0xDEAD_BEEF_u32;
        let original = 42i32;
        let mixed = xor_mix(original, key);
        assert_ne!(mixed, original);
        assert_eq!(xor_mix(mixed, key), original);
    }

    #[test]
    fn known_vector() {
        // value=1, key=0 -> NOT(0)=0xFFFFFFFF, 1 ^ 0xFFFFFFFF = 0xFFFFFFFE = -2i32
        assert_eq!(xor_mix(1, 0), -2);
    }
}
