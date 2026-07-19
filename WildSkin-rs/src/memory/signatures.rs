//! AOB signatures for the game globals and functions this port resolves.
//! `GAME_CLIENT_SIG` is resolved first (`wait_for_game_client`); the rest in
//! the second phase (`resolve_all`). See `super::resolve`.

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
    patterns: &["48 8B 0D ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? E8 ? ? ? ? E8 ? ? ? ? 48 8B 0D ? ? ? ? 48 8B 01"],
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
/// The game's own MSVC string destructor. Matches the destructor's own entry
/// prologue directly (no call-rel32), so `resolve()`'s default branch returns
/// the match address as-is. Used by the Lux/Sona chroma clear path in
/// `check_special_skins` to free a discarded stack element's `model` string.
pub(super) const MSVC_STRING_DTOR_SIG: Signature = Signature {
    patterns: &["F6 41 0C 01 74 08 48 8B 09"],
    sub_base: true,
    read: false,
    relative: false,
    additional: 0,
};

/// Every signature resolved in the second phase (`resolve_all`), once the
/// game has reached `Running`. Mirrors the original `Memory::sigs`, minus
/// `MaterialRegistry::SwapChain` and its `GetSingletonPtr` accessor (see the
/// module-level deviation note in the task plan): hudhook hooks
/// `Present`/`ResizeBuffers` globally and never needs the game's live
/// swapchain instance those two existed to fetch.
#[allow(dead_code, reason = "canonical list of the second-phase signature set, asserted by tests; production `resolve_all` resolves each signature individually rather than iterating this")]
pub(super) static FULL_SIGS: &[&Signature] = &[
    &PLAYER_SIG,
    &HERO_LIST_SIG,
    &CHAMPION_MANAGER_SIG,
    &MINION_LIST_SIG,
    &TURRET_LIST_SIG,
    &WINDOW_SIG,
    &CHARACTER_DATA_STACK_OFFSET_SIG,
    &SKIN_ID_OFFSET_SIG,
    &PUSH_FN_SIG,
    &UPDATE_FN_SIG,
    &TRANSLATE_STRING_FN_SIG,
    &GOLD_REDIRECT_FN_SIG,
    &MSVC_STRING_DTOR_SIG,
];

#[cfg(test)]
mod tests {
    use super::{FULL_SIGS, GAME_CLIENT_SIG};

    #[test]
    fn game_client_signature_is_a_single_pattern() {
        // Locks the shape of the two-phase split: only GameClient resolves
        // in the "wait for Running" phase, everything else waits for it.
        assert_eq!(GAME_CLIENT_SIG.patterns.len(), 1);
    }

    #[test]
    fn full_signature_list_has_thirteen_entries() {
        // Original had 14 in Memory::sigs; this port drops the 2 that only
        // existed to hand-roll a DX11 vtable hook hudhook makes unnecessary
        // (MaterialRegistry::SwapChain + its GetSingletonPtr accessor), and
        // adds 1 the original never needed: MSVC_STRING_DTOR_SIG, used by the
        // Lux/Sona chroma clear path in `check_special_skins`.
        assert_eq!(FULL_SIGS.len(), 13);
    }
}
