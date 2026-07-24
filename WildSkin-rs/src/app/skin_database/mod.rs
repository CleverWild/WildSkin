//! Skin catalogue: static tables, the runtime loader, and shared types.
//!
//! - [`types`]: [`SkinInfo`], [`SpecialSkin`] and friends.
//! - [`tables`]: [`SkinDatabase::empty`], the hardcoded tables.
//! - this file: [`SkinDatabase::load`] (live-memory scan).

mod tables;
mod types;

pub use types::*;

use crate::memory::ResolvedOffsets;
use crate::sdk::champion::ChampionManager;
use crate::util::fnv::fnv1a;
use std::collections::HashMap;

// Field names keep their "_skins" suffix, consistent with call sites in
// gui.rs/skin_logic.rs.
#[allow(
    clippy::struct_field_names,
    reason = "descriptive per-table names, consistent with call sites across gui.rs/skin_logic.rs"
)]
pub struct SkinDatabase {
    pub champions_skins: HashMap<u64, Vec<SkinInfo>>,
    pub wards_skins: Vec<(u32, String)>,
    pub minions_skins: Vec<&'static str>,
    pub turret_skins: Vec<&'static str>,
    pub jungle_mobs_skins: Vec<JungleMobSkinInfo>,
    pub special_skins: Vec<SpecialSkin>,
}

impl SkinDatabase {
    /// # Safety
    /// `champion_manager` points at a live `ChampionManager`, and `offsets`
    /// was resolved against that same live process.
    pub unsafe fn load(
        &mut self,
        champion_manager: *const ChampionManager,
        offsets: &ResolvedOffsets,
    ) {
        // SAFETY: caller guarantees `champion_manager` is a live pointer.
        let manager = unsafe { &*champion_manager };
        // SAFETY: `manager` was just dereferenced above from a live pointer,
        // so its `champions` array is live too.
        let champions = unsafe { manager.champions.as_slice() };
        for &champion_ptr in champions {
            // SAFETY: caller guarantees every entry in the live
            // `ChampionManager::champions` array is a valid `*mut Champion`.
            let champion = unsafe { &*champion_ptr };
            // SAFETY: `champion` is a valid reference into live game memory
            // per the guarantee above; its `skins` array is live too.
            let mut skin_ids: Vec<i32> = unsafe { champion.skins.as_slice() }
                .iter()
                .map(|s| s.skin_id)
                .collect();
            skin_ids.sort_unstable();

            // SAFETY: `champion.champion_name` is a live `AString` inside the
            // same valid `Champion` referenced above.
            let champ_name = unsafe { champion.champion_name.as_str() }.to_owned();
            let champ_hash = fnv1a(&champ_name);
            let mut seen: HashMap<String, i32> = HashMap::new();

            for id in skin_ids {
                let display_key = format!("game_character_skin_displayname_{champ_name}_{id}");
                let translated = if id > 0 {
                    let Ok(c_key) = std::ffi::CString::new(display_key.clone()) else {
                        continue;
                    };
                    // SAFETY: caller guarantees `offsets.fns.translate_string` is
                    // live, matching the requirement of `translate`.
                    let Some(t) = (unsafe { offsets.translate(&c_key) }) else {
                        continue;
                    };
                    t
                } else {
                    champ_name.clone()
                };

                if translated == display_key {
                    continue;
                }

                let name = dedupe_skin_name(&mut seen, &translated);

                self.champions_skins
                    .entry(champ_hash)
                    .or_default()
                    .push(SkinInfo {
                        model_name: champ_name.clone(),
                        skin_name: name,
                        skin_id: id,
                    });

                // Any ChromaSlot entry matching this (champion, skin id)
                // contributes its alternate models as extra champions_skins
                // entries; a new table entry above is picked up automatically.
                for special in &self.special_skins {
                    if let SpecialSkinKind::ChromaSlot { skin_id, models } = &special.kind
                        && special.champ_hash == champ_hash
                        && *skin_id == id
                    {
                        for model_info in models {
                            self.champions_skins
                                .entry(champ_hash)
                                .or_default()
                                .push(SkinInfo {
                                    model_name: model_info.model_name.to_owned(),
                                    skin_name: model_info.label.to_owned(),
                                    skin_id: id,
                                });
                        }
                    }
                }
            }
        }

        for ward_id in 1u32.. {
            let key = format!("game_character_skin_displayname_SightWard_{ward_id}");
            let Ok(c_key) = std::ffi::CString::new(key) else {
                break;
            };
            // SAFETY: caller guarantees `offsets.fns.translate_string` is live.
            let Some(name) = (unsafe { offsets.translate(&c_key) }) else {
                break;
            };
            if name.is_empty() {
                break;
            }
            self.wards_skins.push((ward_id, name));
        }
    }
}

/// Dedupes translated skin display names: first occurrence used as-is, each
/// later duplicate gets " Chroma N" (N = duplicates seen so far). Counter is
/// keyed by the ORIGINAL name, never an already-suffixed one.
fn dedupe_skin_name(seen: &mut HashMap<String, i32>, translated: &str) -> String {
    match seen.get(translated).copied() {
        None => {
            seen.insert(translated.to_owned(), 1);
            translated.to_owned()
        }
        Some(n) => {
            seen.insert(translated.to_owned(), n + 1);
            format!("{translated} Chroma {n}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minions_skins_table_matches_the_original_count_and_order() {
        let db = SkinDatabase::empty();
        assert_eq!(
            db.minions_skins,
            vec![
                "Minion",
                "Summer Minion",
                "Project Minion",
                "Snowdown Minion",
                "Draven Minion",
                "Star Guardian Minion",
                "Arcade Minion",
                "Snowdown 2 Minion",
                "Odyssey Minion",
                "Mouse Minion",
                "Arcane Minion",
            ]
        );
    }

    #[test]
    fn turret_skins_table_has_nineteen_entries() {
        assert_eq!(SkinDatabase::empty().turret_skins.len(), 19);
    }

    #[test]
    fn jungle_mobs_table_has_six_entries_baron_first() {
        let db = SkinDatabase::empty();
        assert_eq!(db.jungle_mobs_skins.len(), 6);
        assert_eq!(db.jungle_mobs_skins[0].name, "Baron");
        assert_eq!(
            db.jungle_mobs_skins[0].name_hashes,
            vec![fnv1a("SRU_Baron")]
        );
        assert_eq!(db.jungle_mobs_skins[0].skins.len(), 8);
    }

    #[test]
    fn krug_has_three_name_hash_variants() {
        let db = SkinDatabase::empty();
        let krug = db
            .jungle_mobs_skins
            .iter()
            .find(|m| m.name == "Krug")
            .unwrap();
        assert_eq!(krug.name_hashes.len(), 3);
    }

    #[test]
    fn special_skins_table_has_thirteen_entries_katarina_first() {
        let db = SkinDatabase::empty();
        assert_eq!(db.special_skins.len(), 13);
        assert_eq!(db.special_skins[0].champ_hash, fnv1a("Katarina"));
        match &db.special_skins[0].kind {
            SpecialSkinKind::GearVariants {
                skin_id_range,
                gears,
                reset_to_zero_on_select,
            } => {
                assert_eq!(*skin_id_range, 29..=36);
                assert_eq!(gears.len(), 6);
                assert!(*reset_to_zero_on_select);
            }
            SpecialSkinKind::ChromaSlot { .. } => panic!("Katarina should be GearVariants"),
        }
    }

    #[test]
    fn ezreal_gear_variants_do_not_reset_to_zero_on_select() {
        let db = SkinDatabase::empty();
        let ezreal = db
            .special_skins
            .iter()
            .find(|s| s.champ_hash == fnv1a("Ezreal"))
            .unwrap();
        match &ezreal.kind {
            SpecialSkinKind::GearVariants {
                reset_to_zero_on_select,
                ..
            } => assert!(!reset_to_zero_on_select),
            SpecialSkinKind::ChromaSlot { .. } => panic!("Ezreal should be GearVariants"),
        }
    }

    #[test]
    fn lux_chroma_slot_has_nine_models_at_skin_id_seven() {
        let db = SkinDatabase::empty();
        let lux = db
            .special_skins
            .iter()
            .find(|s| s.champ_hash == fnv1a("Lux"))
            .unwrap();
        match &lux.kind {
            SpecialSkinKind::ChromaSlot { skin_id, models } => {
                assert_eq!(*skin_id, 7);
                assert_eq!(models.len(), 9);
                assert_eq!(models[0].model_name, "LuxAir");
                assert_eq!(models[0].label, "Elementalist Air Lux");
            }
            SpecialSkinKind::GearVariants { .. } => panic!("Lux should be ChromaSlot"),
        }
    }

    #[test]
    fn gear_variants_for_returns_labels_inside_the_range_and_none_outside_it() {
        let db = SkinDatabase::empty();
        let katarina = db
            .special_skins
            .iter()
            .find(|s| s.champ_hash == fnv1a("Katarina"))
            .unwrap();
        assert_eq!(katarina.gear_variants_for(30).unwrap().len(), 6);
        assert!(katarina.gear_variants_for(100).is_none());
    }

    #[test]
    fn gear_variants_for_returns_none_for_a_chroma_slot_entry() {
        let db = SkinDatabase::empty();
        let lux = db
            .special_skins
            .iter()
            .find(|s| s.champ_hash == fnv1a("Lux"))
            .unwrap();
        assert!(lux.gear_variants_for(7).is_none());
    }

    #[test]
    fn dedupe_first_occurrence_is_unchanged() {
        let mut seen = HashMap::new();
        assert_eq!(dedupe_skin_name(&mut seen, "Classic"), "Classic");
    }

    #[test]
    fn dedupe_second_occurrence_gets_chroma_1() {
        let mut seen = HashMap::new();
        dedupe_skin_name(&mut seen, "Classic");
        assert_eq!(dedupe_skin_name(&mut seen, "Classic"), "Classic Chroma 1");
    }

    #[test]
    fn dedupe_third_occurrence_gets_chroma_2_keyed_by_the_original_name() {
        let mut seen = HashMap::new();
        dedupe_skin_name(&mut seen, "Classic");
        dedupe_skin_name(&mut seen, "Classic");
        // "Chroma 2", not "Classic Chroma 1 Chroma 1": counter stays keyed
        // by "Classic", never the suffixed variant.
        assert_eq!(dedupe_skin_name(&mut seen, "Classic"), "Classic Chroma 2");
    }
}
