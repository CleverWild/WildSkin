//! AOB signatures. `GAME_CLIENT_SIG` resolves first (`wait_for_game_client`),
//! the rest in `resolve_all`.

use super::scanner::Signature;

pub(super) const GAME_CLIENT_SIG: Signature = Signature {
    patterns: &["48 8B 05 ? ? ? ? 48 8B F2 83 78"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};

pub(super) const PLAYER_SIG: Signature = Signature {
    patterns: &["48 8B 3D ? ? ? ? 48 85 FF 74 15 48 81 C7"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const HERO_LIST_SIG: Signature = Signature {
    patterns: &["48 8B 05 ? ? ? ? 48 8B ? 08 8B 40 ? ? 8D ? ? ? 3B ? 74"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const CHAMPION_MANAGER_SIG: Signature = Signature {
    patterns: &["48 8B 0D ? ? ? ? 48 69 D0 ? ? 00 00 48 8B 05"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const MINION_LIST_SIG: Signature = Signature {
    patterns: &[
        "48 8B 0D ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? E8 ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? 48 8B 01",
    ],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const TURRET_LIST_SIG: Signature = Signature {
    patterns: &["48 8B 05 ? ? ? ? 48 8B ? 28 48 85 ? 74"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const WINDOW_SIG: Signature = Signature {
    patterns: &["48 8B 0D ? ? ? ? FF 15 ? ? ? ? 48 8B 05 ? ? ? ?"],
    sub_base: true,
    read: false,
    relative: true,
    additional: 0,
};
pub(super) const CHARACTER_DATA_STACK_OFFSET_SIG: Signature = Signature {
    patterns: &["48 8D 8D ? ? 00 00 44 8B 8C 24 ? ? 00 00"],
    sub_base: false,
    read: true,
    relative: false,
    additional: 0,
};
pub(super) const SKIN_ID_OFFSET_SIG: Signature = Signature {
    patterns: &["88 86 ? ? 00 00 48 89 45 ? 0F B6 45 A8 88 86 ? 13"],
    sub_base: false,
    read: true,
    relative: false,
    additional: 0,
};
pub(super) const PUSH_FN_SIG: Signature = Signature {
    patterns: &["E8 ? ? ? ? 48 8D 8D ? ? 00 00 E8 ? ? ? ? 48 85 C0 74 ? 48 85 ED"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};
pub(super) const UPDATE_FN_SIG: Signature = Signature {
    patterns: &["88 54 24 10 55 53 56 57 41 54 41 55 41 56 41"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};
pub(super) const TRANSLATE_STRING_FN_SIG: Signature = Signature {
    patterns: &["E8 ? ? ? ? 0F 57 DB 4C 8B C0 F3 0F 5A DE"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};
pub(super) const GOLD_REDIRECT_FN_SIG: Signature = Signature {
    patterns: &["E8 ? ? ? ? 4C 3B ? 0F 94 C0"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};
/// MSVC string destructor. Entry-prologue match (no call-rel32), so `resolve()`
/// returns the match as-is. Used by the Lux/Sona chroma clear path.
pub(super) const MSVC_STRING_DTOR_SIG: Signature = Signature {
    patterns: &["F6 41 0C 01 74 08 48 8B 09"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};

/// Symbol name -> signature, for the snapshot drift test's `symbol` lookup.
#[cfg(test)]
const NAMED_SIGS: &[(&str, &Signature)] = &[
    ("GAME_CLIENT_SIG", &GAME_CLIENT_SIG),
    ("PLAYER_SIG", &PLAYER_SIG),
    ("HERO_LIST_SIG", &HERO_LIST_SIG),
    ("CHAMPION_MANAGER_SIG", &CHAMPION_MANAGER_SIG),
    ("MINION_LIST_SIG", &MINION_LIST_SIG),
    ("TURRET_LIST_SIG", &TURRET_LIST_SIG),
    ("WINDOW_SIG", &WINDOW_SIG),
    (
        "CHARACTER_DATA_STACK_OFFSET_SIG",
        &CHARACTER_DATA_STACK_OFFSET_SIG,
    ),
    ("SKIN_ID_OFFSET_SIG", &SKIN_ID_OFFSET_SIG),
    ("PUSH_FN_SIG", &PUSH_FN_SIG),
    ("UPDATE_FN_SIG", &UPDATE_FN_SIG),
    ("TRANSLATE_STRING_FN_SIG", &TRANSLATE_STRING_FN_SIG),
    ("GOLD_REDIRECT_FN_SIG", &GOLD_REDIRECT_FN_SIG),
    ("MSVC_STRING_DTOR_SIG", &MSVC_STRING_DTOR_SIG),
];

#[cfg(test)]
mod tests {
    use super::NAMED_SIGS;
    use std::path::PathBuf;

    #[test]
    fn game_client_signature_is_a_single_pattern() {
        // Two-phase split: only GameClient resolves in phase one.
        assert_eq!(super::GAME_CLIENT_SIG.patterns.len(), 1);
    }

    /// Normalize a hex pattern for whitespace-insensitive compare.
    fn norm(p: &str) -> String {
        p.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Newest `skin-<major>.<minor>.json` under the gitignored
    /// `re-index/`, or `None` if the dir/files are absent.
    fn latest_snapshot() -> Option<PathBuf> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("re-index");
        let mut best: Option<(u64, PathBuf)> = None;
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            let Some(ver) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("skin-"))
                .and_then(|n| n.strip_suffix(".json"))
            else {
                continue;
            };
            let mut it = ver.split('.');
            let (Some(Ok(major)), Some(Ok(minor))) = (
                it.next().map(str::parse::<u64>),
                it.next().map(str::parse::<u64>),
            ) else {
                continue;
            };
            let key = major * 1000 + minor;
            if best.as_ref().is_none_or(|(k, _)| key > *k) {
                best = Some((key, path));
            }
        }
        best.map(|(_, p)| p)
    }

    /// The NEWEST RE-index snapshot's `runtime_signature` patterns must still
    /// match the live `signatures.rs` consts they name. Older snapshots are
    /// historical (the game has moved on) and are intentionally not checked;
    /// the whole snapshot layer lives in a gitignored dir, so on CI (no dir)
    /// this is a no-op. Catches the one window that matters: the current-patch
    /// snapshot silently drifting from the code that actually scans/verifies.
    #[test]
    fn newest_snapshot_runtime_signatures_match_signatures_rs() {
        let Some(path) = latest_snapshot() else {
            return;
        };
        let raw = std::fs::read_to_string(&path).unwrap();
        let db: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let mut mismatches = Vec::new();
        for func in db["functions"].as_array().unwrap_or(&Vec::new()) {
            let rs = &func["anchors"]["runtime_signature"];
            let Some(symbol) = rs["symbol"].as_str() else {
                continue;
            };
            // Entries variously key the pattern as "pattern" or "pattern_short".
            let Some(json_pat) = rs["pattern"]
                .as_str()
                .or_else(|| rs["pattern_short"].as_str())
            else {
                mismatches.push(format!("{symbol}: runtime_signature has no pattern field"));
                continue;
            };
            let Some((_, sig)) = NAMED_SIGS.iter().find(|(n, _)| *n == symbol) else {
                mismatches.push(format!("{symbol}: not a known signatures.rs const"));
                continue;
            };
            if !sig.patterns.iter().any(|p| norm(p) == norm(json_pat)) {
                mismatches.push(format!(
                    "{symbol}: snapshot pattern drifted from signatures.rs\n    snapshot: {}\n    code:     {}",
                    norm(json_pat),
                    sig.patterns.iter().map(|p| norm(p)).collect::<Vec<_>>().join(" | "),
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "newest snapshot {} diverged from signatures.rs ({} issue(s)):\n{}",
            path.display(),
            mismatches.len(),
            mismatches.join("\n"),
        );
    }
}
