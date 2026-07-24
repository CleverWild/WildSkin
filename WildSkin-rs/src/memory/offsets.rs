//! `ResolvedOffsets`: resolved game globals, field offsets, and function
//! addresses, plus the typed accessors to reach them. Built once by
//! `resolve_all`. Bucketed into [`GlobalPointers`], [`FieldOffsets`],
//! [`FnAddresses`].

use crate::sdk::ai_base_common::{AIBaseCommon, AIHero, AIMinionClient, AITurret, GoldRedirectFn};
use crate::sdk::champion::ChampionManager;
use crate::sdk::character_data::{DtorFn, PushFn, SkinApplyFns, UpdateFn};
use crate::sdk::game_state::{self, GameClient};
use crate::sdk::primitives::ManagerTemplate;
use std::ffi::CStr;

// Key-to-localized-string lookup, resolved via a call site (`E8 rel32`).
// Prologue has only short (`rel8`) jumps, no layout-dependent `rel32`, so the
// byte template needs no wildcards.
#[abi_verify_macro::verify_abi(
    pattern = "E8 ? ? ? ? 0F 57 DB 4C 8B C0 F3 0F 5A DE",
    full_signature = "40 53 48 83 EC 30 80 39 00 48 8B C1 8B D1 74 0B 48 FF C0 48 8B D0 80 38 00"
)]
type TranslateFn = unsafe extern "system" fn(key: *const i8) -> *const i8;

/// Resolved global-variable pointers (module base, object lists, singletons).
/// Reached only through [`ResolvedOffsets`]' accessors.
pub(super) struct GlobalPointers {
    /// `base`/`window` resolved for parity but unused: hudhook hooks Present
    /// globally, so the window is never needed.
    #[allow(
        dead_code,
        reason = "resolved for parity; not consumed yet, see field doc above"
    )]
    pub(super) base: usize,
    pub(super) player: usize,
    pub(super) champion_manager: usize,
    #[allow(
        dead_code,
        reason = "resolved for parity; not consumed yet, see the `base` field doc"
    )]
    pub(super) window: usize,
    pub(super) hero_list: usize,
    pub(super) minion_list: usize,
    pub(super) turret_list: usize,
    pub(super) game_client: usize,
}

/// Struct-field byte offsets, read/written directly on live game objects.
pub struct FieldOffsets {
    pub character_data_stack: usize,
    pub skin_id: usize,
}

/// Resolved absolute function addresses, transmuted and called through the
/// `sdk` layer's typed `*Fn` aliases.
pub struct FnAddresses {
    /// push/update/dtor, bundled: always resolved and passed together.
    pub skin_apply: SkinApplyFns,
    pub(super) translate_string: TranslateFn,
    pub get_gold_redirect_target: GoldRedirectFn,
}

impl FnAddresses {
    /// Reinterprets raw addresses as their typed `*Fn` aliases once, so no
    /// call site transmutes again.
    ///
    /// # Safety
    /// Each address must be the game's live corresponding function, matching
    /// the target `*Fn` signature.
    pub(super) unsafe fn from_addrs(
        push: usize,
        update: usize,
        dtor: usize,
        translate: usize,
        gold: usize,
    ) -> Self {
        // SAFETY: per fn contract; `push` matches `PushFn`.
        let push = unsafe { std::mem::transmute::<usize, PushFn>(push) };
        // SAFETY: per fn contract; `update` matches `UpdateFn`.
        let update = unsafe { std::mem::transmute::<usize, UpdateFn>(update) };
        // SAFETY: per fn contract; `dtor` matches `DtorFn`.
        let dtor = unsafe { std::mem::transmute::<usize, DtorFn>(dtor) };
        // SAFETY: per fn contract; `translate` matches `TranslateFn`.
        let translate_string = unsafe { std::mem::transmute::<usize, TranslateFn>(translate) };
        // SAFETY: per fn contract; `gold` matches `GoldRedirectFn`.
        let get_gold_redirect_target =
            unsafe { std::mem::transmute::<usize, GoldRedirectFn>(gold) };
        Self {
            skin_apply: SkinApplyFns { push, update, dtor },
            translate_string,
            get_gold_redirect_target,
        }
    }
}

pub struct ResolvedOffsets {
    globals: GlobalPointers,
    pub fields: FieldOffsets,
    pub fns: FnAddresses,
}

/// Reinterprets a slice of raw game-object pointers as a slice of shared
/// references: `*mut T` and `&T` share layout, so `(ptr, len)` is unchanged.
///
/// # Safety
/// Every pointer must be non-null and point at a live `T` valid for `'a`.
const unsafe fn as_ref_slice<T>(ptrs: &[*mut T]) -> &[&T] {
    // SAFETY: per fn contract; `&T` has the same layout as `*mut T`.
    unsafe { std::slice::from_raw_parts(ptrs.as_ptr().cast::<&T>(), ptrs.len()) }
}

impl ResolvedOffsets {
    pub(super) const fn new(
        globals: GlobalPointers,
        fields: FieldOffsets,
        fns: FnAddresses,
    ) -> Self {
        Self {
            globals,
            fields,
            fns,
        }
    }

    /// Test-only: zeroed globals/fields (plain `usize`s, never dereferenced)
    /// plus non-null dummy fn pointers (fn pointers can't be zero).
    #[cfg(test)]
    pub(crate) fn dummy_for_test() -> Self {
        let dummy = std::ptr::NonNull::<()>::dangling().as_ptr() as usize;
        // SAFETY: globals/fields are plain `usize` (all-zero valid, never read);
        // each fn is a non-null dummy the tests never call.
        unsafe {
            Self {
                globals: std::mem::zeroed(),
                fields: std::mem::zeroed(),
                fns: FnAddresses::from_addrs(dummy, dummy, dummy, dummy, dummy),
            }
        }
    }

    #[expect(
        dead_code,
        reason = "accessor for the parity-only `base` field; kept alongside it for when it's wired up"
    )]
    pub const fn base(&self) -> usize {
        self.globals.base
    }
    #[expect(
        dead_code,
        reason = "accessor for the parity-only `window` field; kept alongside it for when it's wired up"
    )]
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
        let list =
            unsafe { &*(self.globals.minion_list as *const ManagerTemplate<AIMinionClient>) };
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
    /// Caller guarantees `self.fns.translate_string` still points at the
    /// game's live `TranslateString`-style function.
    pub unsafe fn translate(&self, key: &CStr) -> Option<String> {
        // SAFETY: per fn contract.
        let raw = unsafe { (self.fns.translate_string)(key.as_ptr()) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: per fn contract; `func` returns a NUL-terminated C string
        // (or null, checked above).
        Some(
            unsafe { CStr::from_ptr(raw) }
                .to_string_lossy()
                .into_owned(),
        )
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
