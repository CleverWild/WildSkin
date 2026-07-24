//! The hardcoded skin tables: `SkinDatabase::empty()`'s static data.
use super::{ChromaModel, JungleMobSkinInfo, SkinDatabase, SpecialSkin, SpecialSkinKind};
use crate::util::fnv::fnv1a;
use std::collections::HashMap;

impl SkinDatabase {
    pub fn empty() -> Self {
        Self {
            champions_skins: HashMap::new(),
            wards_skins: Vec::new(),
            minions_skins: vec![
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
            ],
            turret_skins: vec![
                "Default Order Turret",
                "Default Chaos Turret",
                "Snow Order Turret",
                "Snow Chaos Turret",
                "Twisted Treeline Order Turret",
                "Twisted Treeline Chaos Turret",
                "URF Order Turret",
                "URF Chaos Turret",
                "Arcade Turret",
                "Temple of Lily and Lotus Turret",
                "Arcane Order Turret",
                "Arcane Chaos Turret",
                "Butcher's Bridge Order Turret",
                "Butcher's Bridge Chaos Turret",
                "Howling Abyss Order Turret",
                "Howling Abyss Chaos Turret",
                "Zaun Order Turret",
                "Piltover Chaos Turret",
                "Black Rose Turret",
            ],
            jungle_mobs_skins: vec![
                JungleMobSkinInfo {
                    name: "Baron",
                    name_hashes: vec![fnv1a("SRU_Baron")],
                    skins: vec![
                        "Baron",
                        "Snowdown Baron",
                        "Championship Baron",
                        "Lunar Revel Baron",
                        "MSI Baron",
                        "Odyssey Baron",
                        "Championship Birthday Baron",
                        "Ruined King Baron",
                    ],
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
                    skins: vec![
                        "Scuttle",
                        "Halloween Light Scuttle",
                        "Halloween Dark Scuttle",
                        "Ruined King Scuttle",
                    ],
                },
                JungleMobSkinInfo {
                    name: "Krug",
                    name_hashes: vec![
                        fnv1a("SRU_Krug"),
                        fnv1a("SRU_KrugMini"),
                        fnv1a("SRU_KrugMiniMini"),
                    ],
                    skins: vec!["Krug", "Dark Krug"],
                },
                JungleMobSkinInfo {
                    name: "Razorbeak",
                    name_hashes: vec![fnv1a("SRU_Razorbeak"), fnv1a("SRU_RazorbeakMini")],
                    skins: vec!["Razorbeak", "Chicken Razorbeak"],
                },
            ],
            special_skins: vec![
                SpecialSkin {
                    champ_hash: fnv1a("Katarina"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 29..=36,
                        gears: vec![
                            "Dagger 1", "Dagger 2", "Dagger 3", "Dagger 4", "Dagger 5", "Dagger 6",
                        ],
                        reset_to_zero_on_select: true,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Renekton"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 26..=32,
                        gears: vec!["Head off", "Head on", "Fins", "Ultimate"],
                        reset_to_zero_on_select: true,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("MissFortune"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 16..=16,
                        gears: vec!["Scarlet fair", "Zero hour", "Royal arms", "Starswarm"],
                        reset_to_zero_on_select: true,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Ezreal"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 5..=5,
                        gears: vec!["Level 1", "Level 2", "Level 3"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Ahri"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 86..=86,
                        gears: vec!["Hall of Legends", "Risen Legend", "Immortalized Legend"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Jinx"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 60..=60,
                        gears: vec!["With hood", "Parallel world", "Without hood"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Sett"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 66..=66,
                        gears: vec!["Blue", "Gold", "Red"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Mordekaiser"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 54..=54,
                        gears: vec!["Sahn-Uzal", "Unconquered King", "Iron Revenant"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Kaisa"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 71..=71,
                        gears: vec!["Hall of Legends", "Risen Legend", "Immortalized Legend"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Morgana"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 80..=80,
                        gears: vec!["Mask1", "Mask2", "Mask3", "Mask4", "Mask5", "Mask6"],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Viego"),
                    kind: SpecialSkinKind::GearVariants {
                        skin_id_range: 43..=43,
                        gears: vec![
                            "Sword1", "Sword2", "Sword3", "Sword4", "Sword5", "Sword6", "Sword7",
                        ],
                        reset_to_zero_on_select: false,
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Lux"),
                    kind: SpecialSkinKind::ChromaSlot {
                        skin_id: 7,
                        models: vec![
                            ChromaModel {
                                model_name: "LuxAir",
                                label: "Elementalist Air Lux",
                            },
                            ChromaModel {
                                model_name: "LuxDark",
                                label: "Elementalist Dark Lux",
                            },
                            ChromaModel {
                                model_name: "LuxFire",
                                label: "Elementalist Fire Lux",
                            },
                            ChromaModel {
                                model_name: "LuxIce",
                                label: "Elementalist Ice Lux",
                            },
                            ChromaModel {
                                model_name: "LuxMagma",
                                label: "Elementalist Magma Lux",
                            },
                            ChromaModel {
                                model_name: "LuxMystic",
                                label: "Elementalist Mystic Lux",
                            },
                            ChromaModel {
                                model_name: "LuxNature",
                                label: "Elementalist Nature Lux",
                            },
                            ChromaModel {
                                model_name: "LuxStorm",
                                label: "Elementalist Storm Lux",
                            },
                            ChromaModel {
                                model_name: "LuxWater",
                                label: "Elementalist Water Lux",
                            },
                        ],
                    },
                },
                SpecialSkin {
                    champ_hash: fnv1a("Sona"),
                    kind: SpecialSkinKind::ChromaSlot {
                        skin_id: 6,
                        models: vec![
                            ChromaModel {
                                model_name: "SonaDJGenre02",
                                label: "DJ Sona 2",
                            },
                            ChromaModel {
                                model_name: "SonaDJGenre03",
                                label: "DJ Sona 3",
                            },
                        ],
                    },
                },
            ],
        }
    }
}
