pub fn derive_key() -> u32 {
    // xor_key is the low 4 bytes of one __rdtsc() reading.
    // SAFETY: `_rdtsc` is always safe on x86_64; it reads the TSC register.
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
