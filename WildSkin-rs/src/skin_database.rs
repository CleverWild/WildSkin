use crate::fnv::fnv1a;
use crate::memory::ResolvedOffsets;
use crate::sdk::champion::ChampionManager;
use std::collections::HashMap;
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
        /// `skin_id_range` must explicitly set `gear = 0` (the `-1`
        /// sentinel doesn't render correctly for these three specifically).
        /// false for every other `GearVariants` entry, which gets no
        /// `check_special_skins` behavior beyond the universal `gear = -1`
        /// reset — those entries only matter for the GUI combo and Ctrl+5
        /// cycling.
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
    /// Returns this entry's gear labels if `skin` falls within its
    /// `GearVariants` range; `None` for a non-matching skin id or for a
    /// `ChromaSlot` entry (which has no gear values to cycle/pick).
    pub fn gear_variants_for(&self, skin: i32) -> Option<&[&'static str]> {
        match &self.kind {
            SpecialSkinKind::GearVariants { skin_id_range, gears, .. }
                if skin_id_range.contains(&skin) =>
            {
                Some(gears)
            }
            _ => None,
        }
    }
}

// Field names deliberately keep their "_skins" suffix (matching what each
// table actually holds — champions', wards', minions', etc.) rather than
// dropping it for brevity; renaming would touch every call site across
// `gui.rs`/`skin_logic.rs` for a pure naming nit with no behavior change.
#[allow(clippy::struct_field_names, reason = "descriptive per-table names, consistent with call sites across gui.rs/skin_logic.rs")]
pub struct SkinDatabase {
    pub champions_skins: HashMap<u64, Vec<SkinInfo>>,
    pub wards_skins: Vec<(u32, String)>,
    pub minions_skins: Vec<&'static str>,
    pub turret_skins: Vec<&'static str>,
    pub jungle_mobs_skins: Vec<JungleMobSkinInfo>,
    pub special_skins: Vec<SpecialSkin>,
}

impl SkinDatabase {
    pub fn empty() -> Self {
        Self {
            champions_skins: HashMap::new(),
            wards_skins: Vec::new(),
            minions_skins: vec![
                "Minion", "Summer Minion", "Project Minion", "Snowdown Minion",
                "Draven Minion", "Star Guardian Minion", "Arcade Minion",
                "Snowdown 2 Minion", "Odyssey Minion", "Mouse Minion", "Arcane Minion",
            ],
            turret_skins: vec![
                "Default Order Turret", "Default Chaos Turret",
                "Snow Order Turret", "Snow Chaos Turret",
                "Twisted Treeline Order Turret", "Twisted Treeline Chaos Turret",
                "URF Order Turret", "URF Chaos Turret",
                "Arcade Turret",
                "Temple of Lily and Lotus Turret",
                "Arcane Order Turret", "Arcane Chaos Turret",
                "Butcher's Bridge Order Turret", "Butcher's Bridge Chaos Turret",
                "Howling Abyss Order Turret", "Howling Abyss Chaos Turret",
                "Zaun Order Turret", "Piltover Chaos Turret",
                "Black Rose Turret",
            ],
            jungle_mobs_skins: vec![
                JungleMobSkinInfo {
                    name: "Baron",
                    name_hashes: vec![fnv1a("SRU_Baron")],
                    skins: vec!["Baron", "Snowdown Baron", "Championship Baron", "Lunar Revel Baron", "MSI Baron", "Odyssey Baron", "Championship Birthday Baron", "Ruined King Baron"],
                },
                JungleMobSkinInfo {
                    name: "Blue",
                    name_hashes: vec![fnv1a("SRU_Blue")],
                    skins: vec!["Blue", "Dark Blue", "Pool Party Blue", "Ruined King Blue"],
                },
                JungleMobSkinInfo {
                    name: "Red",
                    name_hashes: vec![fnv1a("SRU_Red")],
                    skins: vec!["Red", "Pool Party Red", "Ruined King Red"],
                },
                JungleMobSkinInfo {
                    name: "Scuttle",
                    name_hashes: vec![fnv1a("Sru_Crab")],
                    skins: vec!["Scuttle", "Halloween Light Scuttle", "Halloween Dark Scuttle", "Ruined King Scuttle"],
                },
                JungleMobSkinInfo {
                    name: "Krug",
                    name_hashes: vec![fnv1a("SRU_Krug"), fnv1a("SRU_KrugMini"), fnv1a("SRU_KrugMiniMini")],
                    skins: vec!["Krug", "Dark Krug"],
                },
                JungleMobSkinInfo {
                    name: "Razorbeak",
                    name_hashes: vec![fnv1a("SRU_Razorbeak"), fnv1a("SRU_RazorbeakMini")],
                    skins: vec!["Razorbeak", "Chicken Razorbeak"],
                },
            ],
            special_skins: vec![
                SpecialSkin { champ_hash: fnv1a("Katarina"), kind: SpecialSkinKind::GearVariants { skin_id_range: 29..=36, gears: vec!["Dagger 1", "Dagger 2", "Dagger 3", "Dagger 4", "Dagger 5", "Dagger 6"], reset_to_zero_on_select: true } },
                SpecialSkin { champ_hash: fnv1a("Renekton"), kind: SpecialSkinKind::GearVariants { skin_id_range: 26..=32, gears: vec!["Head off", "Head on", "Fins", "Ultimate"], reset_to_zero_on_select: true } },
                SpecialSkin { champ_hash: fnv1a("MissFortune"), kind: SpecialSkinKind::GearVariants { skin_id_range: 16..=16, gears: vec!["Scarlet fair", "Zero hour", "Royal arms", "Starswarm"], reset_to_zero_on_select: true } },
                SpecialSkin { champ_hash: fnv1a("Ezreal"), kind: SpecialSkinKind::GearVariants { skin_id_range: 5..=5, gears: vec!["Level 1", "Level 2", "Level 3"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Ahri"), kind: SpecialSkinKind::GearVariants { skin_id_range: 86..=86, gears: vec!["Hall of Legends", "Risen Legend", "Immortalized Legend"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Jinx"), kind: SpecialSkinKind::GearVariants { skin_id_range: 60..=60, gears: vec!["With hood", "Parallel world", "Without hood"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Sett"), kind: SpecialSkinKind::GearVariants { skin_id_range: 66..=66, gears: vec!["Blue", "Gold", "Red"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Mordekaiser"), kind: SpecialSkinKind::GearVariants { skin_id_range: 54..=54, gears: vec!["Sahn-Uzal", "Unconquered King", "Iron Revenant"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Kaisa"), kind: SpecialSkinKind::GearVariants { skin_id_range: 71..=71, gears: vec!["Hall of Legends", "Risen Legend", "Immortalized Legend"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Morgana"), kind: SpecialSkinKind::GearVariants { skin_id_range: 80..=80, gears: vec!["Mask1", "Mask2", "Mask3", "Mask4", "Mask5", "Mask6"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Viego"), kind: SpecialSkinKind::GearVariants { skin_id_range: 43..=43, gears: vec!["Sword1", "Sword2", "Sword3", "Sword4", "Sword5", "Sword6", "Sword7"], reset_to_zero_on_select: false } },
                SpecialSkin { champ_hash: fnv1a("Lux"), kind: SpecialSkinKind::ChromaSlot { skin_id: 7, models: vec![
                    ChromaModel { model_name: "LuxAir", label: "Elementalist Air Lux" },
                    ChromaModel { model_name: "LuxDark", label: "Elementalist Dark Lux" },
                    ChromaModel { model_name: "LuxFire", label: "Elementalist Fire Lux" },
                    ChromaModel { model_name: "LuxIce", label: "Elementalist Ice Lux" },
                    ChromaModel { model_name: "LuxMagma", label: "Elementalist Magma Lux" },
                    ChromaModel { model_name: "LuxMystic", label: "Elementalist Mystic Lux" },
                    ChromaModel { model_name: "LuxNature", label: "Elementalist Nature Lux" },
                    ChromaModel { model_name: "LuxStorm", label: "Elementalist Storm Lux" },
                    ChromaModel { model_name: "LuxWater", label: "Elementalist Water Lux" },
                ] } },
                SpecialSkin { champ_hash: fnv1a("Sona"), kind: SpecialSkinKind::ChromaSlot { skin_id: 6, models: vec![
                    ChromaModel { model_name: "SonaDJGenre02", label: "DJ Sona 2" },
                    ChromaModel { model_name: "SonaDJGenre03", label: "DJ Sona 3" },
                ] } },
            ],
        }
    }

    /// # Safety
    /// Caller guarantees `champion_manager` points at a live `ChampionManager`
    /// inside the game's memory, and `offsets` was resolved against that same
    /// live process (see `ResolvedOffsets::translate`).
    pub unsafe fn load(&mut self, champion_manager: *const ChampionManager, offsets: &ResolvedOffsets) {
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
            let mut skin_ids: Vec<i32> =
                unsafe { champion.skins.as_slice() }.iter().map(|s| s.skin_id).collect();
            skin_ids.sort_unstable();

            // SAFETY: `champion.champion_name` is a live `AString` inside the
            // same valid `Champion` referenced above.
            let champ_name = unsafe { champion.champion_name.as_str() }.to_owned();
            let champ_hash = fnv1a(&champ_name);
            let mut seen: HashMap<String, i32> = HashMap::new();

            for id in skin_ids {
                let display_key = format!("game_character_skin_displayname_{champ_name}_{id}");
                let translated = if id > 0 {
                    let Ok(c_key) = std::ffi::CString::new(display_key.clone()) else { continue };
                    // SAFETY: caller guarantees `offsets.fns.translate_string` is
                    // live, matching the requirement of `translate`.
                    let Some(t) = (unsafe { offsets.translate(&c_key) }) else { continue };
                    t
                } else {
                    champ_name.clone()
                };

                if translated == display_key {
                    continue;
                }

                let name = dedupe_skin_name(&mut seen, &translated);

                self.champions_skins.entry(champ_hash).or_default().push(SkinInfo {
                    model_name: champ_name.clone(),
                    skin_name: name,
                    skin_id: id,
                });

                // Generic chroma-slot insertion: any `special_skins` entry
                // whose `ChromaSlot::skin_id` matches this exact
                // (champion, skin id) pair contributes its alternate models
                // as additional `champions_skins` entries. A future
                // chroma-slot champion needs only a new table entry above —
                // this loop picks it up automatically.
                for special in &self.special_skins {
                    if let SpecialSkinKind::ChromaSlot { skin_id, models } = &special.kind
                        && special.champ_hash == champ_hash
                        && *skin_id == id
                    {
                        for model_info in models {
                            self.champions_skins.entry(champ_hash).or_default().push(SkinInfo {
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
            let Ok(c_key) = std::ffi::CString::new(key) else { break };
            // SAFETY: caller guarantees `offsets.fns.translate_string` is live.
            let Some(name) = (unsafe { offsets.translate(&c_key) }) else { break };
            if name.is_empty() {
                break;
            }
            self.wards_skins.push((ward_id, name));
        }
    }
}

/// Deduplicates a champion's translated skin display names the way the
/// original does: the FIRST time a name is seen it's used as-is; every
/// later duplicate (a chroma sharing its base skin's display name) gets
/// " Chroma N" appended, where N is how many duplicates have been seen
/// so far. The counter is always keyed by the ORIGINAL untouched name,
/// never by an already-suffixed one — getting this backwards was an easy
/// mistake caught in this task's own design review.
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
                "Minion", "Summer Minion", "Project Minion", "Snowdown Minion",
                "Draven Minion", "Star Guardian Minion", "Arcade Minion",
                "Snowdown 2 Minion", "Odyssey Minion", "Mouse Minion", "Arcane Minion",
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
        assert_eq!(db.jungle_mobs_skins[0].name_hashes, vec![fnv1a("SRU_Baron")]);
        assert_eq!(db.jungle_mobs_skins[0].skins.len(), 8);
    }

    #[test]
    fn krug_has_three_name_hash_variants() {
        let db = SkinDatabase::empty();
        let krug = db.jungle_mobs_skins.iter().find(|m| m.name == "Krug").unwrap();
        assert_eq!(krug.name_hashes.len(), 3);
    }

    #[test]
    fn special_skins_table_has_thirteen_entries_katarina_first() {
        let db = SkinDatabase::empty();
        assert_eq!(db.special_skins.len(), 13);
        assert_eq!(db.special_skins[0].champ_hash, fnv1a("Katarina"));
        match &db.special_skins[0].kind {
            SpecialSkinKind::GearVariants { skin_id_range, gears, reset_to_zero_on_select } => {
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
        let ezreal = db.special_skins.iter().find(|s| s.champ_hash == fnv1a("Ezreal")).unwrap();
        match &ezreal.kind {
            SpecialSkinKind::GearVariants { reset_to_zero_on_select, .. } => assert!(!reset_to_zero_on_select),
            SpecialSkinKind::ChromaSlot { .. } => panic!("Ezreal should be GearVariants"),
        }
    }

    #[test]
    fn lux_chroma_slot_has_nine_models_at_skin_id_seven() {
        let db = SkinDatabase::empty();
        let lux = db.special_skins.iter().find(|s| s.champ_hash == fnv1a("Lux")).unwrap();
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
        let katarina = db.special_skins.iter().find(|s| s.champ_hash == fnv1a("Katarina")).unwrap();
        assert_eq!(katarina.gear_variants_for(30).unwrap().len(), 6);
        assert!(katarina.gear_variants_for(100).is_none());
    }

    #[test]
    fn gear_variants_for_returns_none_for_a_chroma_slot_entry() {
        let db = SkinDatabase::empty();
        let lux = db.special_skins.iter().find(|s| s.champ_hash == fnv1a("Lux")).unwrap();
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
        // Must be "Chroma 2", not "Classic Chroma 1 Chroma 1" — the counter
        // stays keyed by "Classic", never by the already-suffixed variant.
        assert_eq!(dedupe_skin_name(&mut seen, "Classic"), "Classic Chroma 2");
    }
}
