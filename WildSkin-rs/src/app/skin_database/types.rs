//! Shared skin-catalogue types, re-exported from the parent module.
use std::ops::RangeInclusive;

pub struct SkinInfo {
    pub model_name: String,
    pub skin_name: String,
    pub skin_id: i32,
}

pub struct JungleMobSkinInfo {
    pub name: &'static str,
    pub name_hashes: Vec<u64>,
    pub skins: Vec<&'static str>,
}

pub struct ChromaModel {
    pub model_name: &'static str,
    pub label: &'static str,
}

pub enum SpecialSkinKind {
    /// A numeric `gear` byte selects among visual variants of the same
    /// `skin_id` (Katarina's daggers, Renekton's forms, etc.).
    GearVariants {
        skin_id_range: RangeInclusive<i32>,
        gears: Vec<&'static str>,
        /// true only for Katarina/Renekton/MissFortune: selecting a skin in
        /// `skin_id_range` must set `gear = 0` (the `-1` sentinel doesn't
        /// render for these three). false elsewhere.
        reset_to_zero_on_select: bool,
    },
    /// Selecting this exact `skin_id` is a full alternate-model clear+push,
    /// not a numeric gear value (Lux's elemental forms, Sona's DJ forms).
    ChromaSlot {
        skin_id: i32,
        models: Vec<ChromaModel>,
    },
}

pub struct SpecialSkin {
    pub champ_hash: u64,
    pub kind: SpecialSkinKind,
}

impl SpecialSkin {
    /// Gear labels if `skin` is in the `GearVariants` range; `None` for a
    /// non-matching id or a `ChromaSlot` entry.
    pub fn gear_variants_for(&self, skin: i32) -> Option<&[&'static str]> {
        match &self.kind {
            SpecialSkinKind::GearVariants {
                skin_id_range,
                gears,
                ..
            } if skin_id_range.contains(&skin) => Some(gears),
            _ => None,
        }
    }
}
