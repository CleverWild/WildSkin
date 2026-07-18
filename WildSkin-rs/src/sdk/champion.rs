use super::primitives::{AString, RiotArray};

// These field names deliberately match the reverse-engineered game/original
// C++ field names (skin_id, skin_name, champion_name) rather than clippy's
// preferred shorter names — renaming them would reduce traceability against
// the original layout these structs mirror.
crate::offset!(
    #[allow(clippy::struct_field_names, reason = "field names intentionally match the original reverse-engineered layout")]
    pub struct Skin {
        0x0 => pub skin_id: i32,
        0x8 => pub skin_name: AString,
    }
);

crate::offset!(
    #[allow(clippy::struct_field_names, reason = "field names intentionally match the original reverse-engineered layout")]
    pub struct Champion {
        0x8  => pub champion_name: AString,
        0xC8 => pub skins: RiotArray<Skin>,
    }
);

crate::offset!(
    pub struct ChampionManager {
        0x18 => pub champions: RiotArray<*mut Champion>,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn champion_name_lands_right_after_the_8_byte_pad() {
        assert_eq!(std::mem::offset_of!(Champion, champion_name), 0x8);
    }

    #[test]
    fn skins_array_lands_at_the_expected_offset() {
        // 0x8 (pad) + 16 (AString) + 0xB0 (pad) = 0xC8
        assert_eq!(std::mem::offset_of!(Champion, skins), 0x8 + 16 + 0xB0);
    }

    #[test]
    fn champion_manager_champions_array_lands_after_0x18_pad() {
        assert_eq!(std::mem::offset_of!(ChampionManager, champions), 0x18);
    }
}
