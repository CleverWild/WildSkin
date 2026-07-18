//! `ResolvedOffsets`: the resolved game globals and function addresses, plus
//! the typed accessors the rest of the crate uses to reach them. Constructed
//! once by `super::resolve::resolve_all`.
//!
//! Bucketed by *how they're used*, rather than the original's single flat
//! `offsets::` namespace: [`GlobalPointers`] (global-variable pointers),
//! [`FieldOffsets`] (struct-field byte offsets), [`FnAddresses`] (absolute
//! function addresses).

use crate::sdk::ai_base_common::{AIBaseCommon, AIHero, AIMinionClient, AITurret};
use crate::sdk::champion::ChampionManager;
use crate::sdk::game_state::{self, GameClient};
use crate::sdk::primitives::ManagerTemplate;
use std::ffi::CStr;

// The game's key-to-localized-string lookup. Resolved via a call site
// (`call TranslateString; ...`), so `call_target = true` follows the
// `E8 rel32` to the real function. Its prologue here has only short (`rel8`)
// in-function jumps, no layout-dependent `rel32` — so the byte template
// needs no wildcards.
#[abi_verify_macro::verify_abi(
    pattern = "E8 ? ? ? ? 0F 57 DB 4C 8B C0 F3 0F 5A DE",
    expected_args = 1,
    full_signature = "40 53 48 83 EC 30 80 39 00 48 8B C1 8B D1 74 0B 48 FF C0 48 8B D0 80 38 00"
)]
type TranslateFn = unsafe extern "system" fn(key: *const i8) -> *const i8;

/// Resolved global-variable pointers: the module base plus the game's global
/// object lists and singletons. Reached only through [`ResolvedOffsets`]'
/// accessor methods.
struct GlobalPointers {
    /// `base` and `window` are resolved for parity with the original's offset
    /// set but not consumed yet — hudhook hooks Present globally, so the port
    /// never needs the game window.
    #[allow(dead_code, reason = "resolved for parity; not consumed yet — see field doc above")]
    base: usize,
    player: usize,
    champion_manager: usize,
    #[allow(dead_code, reason = "resolved for parity; not consumed yet — see the `base` field doc")]
    window: usize,
    hero_list: usize,
    minion_list: usize,
    turret_list: usize,
    game_client: usize,
}

/// Resolved struct-field byte offsets, read/written directly on live game
/// objects.
pub struct FieldOffsets {
    pub character_data_stack: usize,
    pub skin_id: usize,
}

/// Resolved absolute function addresses, transmuted and called through the
/// `sdk` layer's typed `*Fn` aliases.
pub struct FnAddresses {
    pub character_data_stack_push: usize,
    pub character_data_stack_update: usize,
    pub(super) translate_string: usize,
    pub get_gold_redirect_target: usize,
    pub msvc_string_dtor: usize,
}

pub struct ResolvedOffsets {
    globals: GlobalPointers,
    pub fields: FieldOffsets,
    pub fns: FnAddresses,
}

/// Reinterprets a slice of raw game-object pointers as a slice of shared
/// references. `*mut T` and `&T` are both thin pointers with identical size
/// and alignment, so the slice's `(ptr, len)` layout is unchanged.
///
/// # Safety
/// Every pointer in `ptrs` must be non-null and point at a live, valid `T`
/// that stays valid for `'a` — exactly `&T`'s own validity requirement.
const unsafe fn as_ref_slice<T>(ptrs: &[*mut T]) -> &[&T] {
    // SAFETY: per fn contract; `&T` has the same layout as `*mut T`.
    unsafe { std::slice::from_raw_parts(ptrs.as_ptr().cast::<&T>(), ptrs.len()) }
}

impl ResolvedOffsets {
    #[allow(clippy::too_many_arguments, reason = "one-shot constructor fed by resolve_all's flat set of resolved values; a builder type would just move the same 15 values around")]
    pub(super) const fn new(
        base: usize,
        player: usize,
        champion_manager: usize,
        window: usize,
        hero_list: usize,
        minion_list: usize,
        turret_list: usize,
        game_client: usize,
        character_data_stack: usize,
        skin_id: usize,
        character_data_stack_push: usize,
        character_data_stack_update: usize,
        translate_string: usize,
        get_gold_redirect_target: usize,
        msvc_string_dtor: usize,
    ) -> Self {
        Self {
            globals: GlobalPointers {
                base,
                player,
                champion_manager,
                window,
                hero_list,
                minion_list,
                turret_list,
                game_client,
            },
            fields: FieldOffsets {
                character_data_stack,
                skin_id,
            },
            fns: FnAddresses {
                character_data_stack_push,
                character_data_stack_update,
                translate_string,
                get_gold_redirect_target,
                msvc_string_dtor,
            },
        }
    }

    #[expect(dead_code, reason = "accessor for the parity-only `base` field; kept alongside it for when it's wired up")]
    pub const fn base(&self) -> usize {
        self.globals.base
    }
    #[expect(dead_code, reason = "accessor for the parity-only `window` field; kept alongside it for when it's wired up")]
    pub const fn window(&self) -> usize {
        self.globals.window
    }

    pub const fn player(&self) -> Option<*mut AIBaseCommon> {
        if self.globals.player == 0 {
            None
        } else {
            Some(self.globals.player as *mut AIBaseCommon)
        }
    }

    /// # Safety
    /// Caller guarantees `self.globals.player` still points at a live
    /// `AIBaseCommon`, if non-null.
    pub const unsafe fn player_ref(&self) -> Option<&AIBaseCommon> {
        match self.player() {
            // SAFETY: per fn contract.
            Some(p) => Some(unsafe { &*p }),
            None => None,
        }
    }

    /// # Safety
    /// Caller guarantees `self.globals.hero_list` still points at a live
    /// `ManagerTemplate<AIHero>` whose entries are all live `AIHero`s.
    pub const unsafe fn hero_list(&self) -> &[&AIHero] {
        // SAFETY: per fn contract.
        let list = unsafe { &*(self.globals.hero_list as *const ManagerTemplate<AIHero>) };
        // SAFETY: per fn contract.
        let ptrs = unsafe { list.as_slice() };
        // SAFETY: per fn contract.
        unsafe { as_ref_slice(ptrs) }
    }

    /// # Safety
    /// Caller guarantees `self.globals.minion_list` still points at a live
    /// `ManagerTemplate<AIMinionClient>` whose entries are all live.
    pub const unsafe fn minion_list(&self) -> &[&AIMinionClient] {
        // SAFETY: per fn contract.
        let list = unsafe { &*(self.globals.minion_list as *const ManagerTemplate<AIMinionClient>) };
        // SAFETY: per fn contract.
        let ptrs = unsafe { list.as_slice() };
        // SAFETY: per fn contract.
        unsafe { as_ref_slice(ptrs) }
    }

    /// # Safety
    /// Caller guarantees `self.globals.turret_list` still points at a live
    /// `ManagerTemplate<AITurret>` whose entries are all live `AITurret`s.
    pub const unsafe fn turret_list(&self) -> &[&AITurret] {
        // SAFETY: per fn contract.
        let list = unsafe { &*(self.globals.turret_list as *const ManagerTemplate<AITurret>) };
        // SAFETY: per fn contract.
        let ptrs = unsafe { list.as_slice() };
        // SAFETY: per fn contract.
        unsafe { as_ref_slice(ptrs) }
    }

    pub const unsafe fn champion_manager(&self) -> *const ChampionManager {
        self.globals.champion_manager as *const ChampionManager
    }

    /// # Safety
    /// Caller guarantees `self.globals.game_client` still points at a live
    /// `GameClient` inside the game's memory.
    pub const unsafe fn is_running(&self) -> bool {
        // SAFETY: per fn contract.
        unsafe { game_state::is_running(self.globals.game_client as *const GameClient) }
    }

    /// # Safety
    /// Caller guarantees `self.fns.translate_string` is the address of the
    /// game's live `TranslateString`-style function, matching `TranslateFn`.
    pub unsafe fn translate(&self, key: &CStr) -> Option<String> {
        // SAFETY: per fn contract.
        let func: TranslateFn = unsafe { std::mem::transmute(self.fns.translate_string) };
        // SAFETY: per fn contract.
        let raw = unsafe { func(key.as_ptr()) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: per fn contract; `func` returns a NUL-terminated C string
        // (or null, checked above).
        Some(unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::as_ref_slice;

    #[test]
    fn as_ref_slice_yields_references_to_the_same_objects() {
        let mut a = 10i32;
        let mut b = 20i32;
        let pa: *mut i32 = &raw mut a;
        let pb: *mut i32 = &raw mut b;
        let ptrs = [pa, pb];
        // SAFETY: both pointers are non-null and point at the live `a`/`b`.
        let refs = unsafe { as_ref_slice(&ptrs) };
        assert_eq!(refs.len(), 2);
        assert_eq!(*refs[0], 10);
        assert_eq!(*refs[1], 20);
        // Same objects, not copies: identity is preserved through the reinterpret.
        assert!(std::ptr::eq(refs[0], &raw const a));
        assert!(std::ptr::eq(refs[1], &raw const b));
    }

    #[test]
    fn as_ref_slice_of_empty_is_empty() {
        let ptrs: [*mut i32; 0] = [];
        // SAFETY: an empty slice dereferences nothing.
        let refs = unsafe { as_ref_slice(&ptrs) };
        assert!(refs.is_empty());
    }
}
