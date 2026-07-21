use super::character_data::CharacterDataStack;
use super::game_object::GameObject;
use std::ffi::CStr;
use std::ops::Deref;

use crate::skin_database::{SpecialSkin, SpecialSkinKind};

// The game's minion gold-redirect-target lookup. Resolved via a call site
// (`call GoldRedirectTarget; cmp ...; sete al`), so `call_target = true`
// follows the `E8 rel32`. The resolved entry is a one-argument tail-call
// thunk: `add rcx, 0x6F8; jmp <real fn>` — the `E9 rel32`'s displacement is
// layout-dependent, so its four bytes are wildcarded in the byte template.
#[abi_verify_macro::verify_abi(
    pattern = "E8 ? ? ? ? 4C 3B ? 0F 94 C0",
    expected_args = 1,
    full_signature = "48 81 C1 F8 06 00 00 E9 ? ? ? ? CC CC CC CC"
)]
type GoldRedirectFn = unsafe extern "system" fn(this: usize) -> *mut AIBaseCommon;

/// Mirrors the game's live `xor_value<int32_t>` field (24 bytes). Calling
/// `encrypt` on an instance obtained via a pointer into the game's own
/// memory mutates that live field directly.
#[repr(C)]
pub struct GameXorSlot {
    xor_key: u32,
    values_table: [i32; 4],
    key_initialized: u8,
    bytes_xor_count: u8,
    bytes_xor_count_8: u8,
    value_index: u8,
}

impl GameXorSlot {
    /// Mirrors `xor_value<int32_t>::encrypt`. `decrypt()` is intentionally
    /// not ported — see crypt.rs's module doc for why.
    ///
    /// # Safety
    /// Caller guarantees `self` is a valid, live `GameXorSlot`.
    pub unsafe fn encrypt(&mut self, value: i32) {
        if self.key_initialized == 0 {
            self.xor_key = crate::crypt::derive_key();
            // sizeof(i32): 4/4 whole u32 words, 0 trailing bytes — the game's
            // `decrypt` reads these to undo the xor, so we must set them too.
            self.bytes_xor_count = 1;
            self.bytes_xor_count_8 = 0;
            self.key_initialized = 1;
            self.value_index = 0;
        }
        let mixed = crate::crypt::xor_mix(value, self.xor_key);
        let new_index = (self.value_index + 1) & 3;
        self.values_table[new_index as usize] = mixed;
        self.value_index = new_index;
    }
}

#[repr(C)]
pub struct AIBaseCommon {
    pub base: GameObject,
}

impl Deref for AIBaseCommon {
    type Target = GameObject;
    fn deref(&self) -> &GameObject {
        &self.base
    }
}

impl AIBaseCommon {
    /// # Safety
    /// Caller guarantees `self` is a live `AIBaseCommon`, `cds_offset` is the
    /// correct byte offset to its `CharacterDataStack` field, and the
    /// resulting pointer is 8-byte aligned.
    const unsafe fn character_data_stack(&self, cds_offset: usize) -> *mut CharacterDataStack {
        let base = std::ptr::from_ref(self);

        // SAFETY: per fn contract.
        unsafe {
            base.byte_add(cds_offset)
                .cast_mut()
                .cast::<CharacterDataStack>()
        }
    }

    /// Same as [`character_data_stack`](Self::character_data_stack), but
    /// returns the dereferenced `&CharacterDataStack` directly — centralizing
    /// the `unsafe { &*ptr }` every caller would otherwise repeat.
    ///
    /// # Safety
    /// Same contract as [`character_data_stack`](Self::character_data_stack).
    pub unsafe fn character_data_stack_ref(&self, cds_offset: usize) -> &CharacterDataStack {
        // SAFETY: per fn contract.
        let ptr = unsafe { self.character_data_stack(cds_offset) };
        // SAFETY: per fn contract.
        unsafe { &*ptr }
    }

    /// Mutable counterpart to
    /// [`character_data_stack_ref`](Self::character_data_stack_ref).
    ///
    /// # Safety
    /// Same contract as [`character_data_stack`](Self::character_data_stack),
    /// plus: caller guarantees no other live reference (shared or mutable)
    /// into the same `CharacterDataStack` exists for the duration of the
    /// returned borrow.
    #[allow(
        clippy::mut_from_ref,
        reason = "`self` is itself a reference conjured into live, externally-mutated game memory; the no-aliasing contract is the caller's, per the # Safety doc"
    )]
    pub unsafe fn character_data_stack_mut(&self, cds_offset: usize) -> &mut CharacterDataStack {
        // SAFETY: per fn contract.
        let ptr = unsafe { self.character_data_stack(cds_offset) };
        // SAFETY: per fn contract.
        unsafe { &mut *ptr }
    }

    /// # Safety
    /// Caller guarantees `self` is live, `cds_offset` is correct, and
    /// `push_fn`/`dtor_fn` are the addresses of the game's
    /// `CharacterDataStack::push` and its MSVC string destructor.
    unsafe fn check_special_skins(
        &self,
        cds_offset: usize,
        push_fn: usize,
        dtor_fn: usize,
        model: &CStr,
        skin: i32,
        special_skins: &[SpecialSkin],
    ) -> bool {
        // SAFETY: per fn contract.
        let stack = unsafe { self.character_data_stack_mut(cds_offset) };
        // SAFETY: per fn contract.
        let champ_hash = crate::fnv::fnv1a(unsafe { stack.base_skin.model.as_str() });

        let kind = special_skins
            .iter()
            .find(|s| s.champ_hash == champ_hash)
            .map(|s| &s.kind);

        match kind {
            Some(SpecialSkinKind::GearVariants {
                skin_id_range,
                reset_to_zero_on_select: true,
                ..
            }) if skin_id_range.contains(&skin) => {
                stack.base_skin.gear = 0;
            }
            Some(SpecialSkinKind::ChromaSlot { skin_id, .. }) if skin == *skin_id => {
                // SAFETY: per fn contract.
                unsafe {
                    stack.clear_stack_properly(dtor_fn);
                }
                // SAFETY: per fn contract.
                unsafe {
                    stack.push(push_fn, model, skin);
                }
                return true;
            }
            Some(SpecialSkinKind::ChromaSlot { .. }) => {
                // Same champion (Lux/Sona), but not their chroma-slot skin.
                // SAFETY: per fn contract.
                unsafe {
                    stack.clear_stack_properly(dtor_fn);
                }
            }
            _ => {
                if stack.base_skin.gear != -1 && champ_hash != crate::fnv::fnv1a("Kayn") {
                    stack.base_skin.gear = -1;
                }
            }
        }
        false
    }

    /// Applies `skin`/`model` to this unit, mirroring the reference's 16.14
    /// `change_skin`: encrypt the skin id into the xor slot, set
    /// `base_skin.skin`, then — unless `check_special_skins` handled it —
    /// `update(true)`. (The Lux/Sona chroma cases in `check_special_skins` still
    /// clear+push.)
    ///
    /// # Safety
    /// Caller guarantees `self` is a live `AIBaseCommon` in the game's
    /// memory, `cds_offset`/`xor_offset` are the correct byte offsets to its
    /// `CharacterDataStack` and `GameXorSlot` fields, and
    /// `push_fn`/`update_fn`/`dtor_fn` are the addresses of the game's
    /// `CharacterDataStack::push`/`::update` and its MSVC string destructor.
    #[allow(
        clippy::too_many_arguments,
        reason = "resolved offsets/fn addresses threaded through from the caller; a builder type would just move the same 8 values around"
    )]
    pub unsafe fn change_skin(
        &self,
        cds_offset: usize,
        xor_offset: usize,
        push_fn: usize,
        update_fn: usize,
        dtor_fn: usize,
        model: &CStr,
        skin: i32,
        special_skins: &[SpecialSkin],
    ) {
        // SAFETY: per fn contract.
        let stack = unsafe { self.character_data_stack_mut(cds_offset) };
        let xor_slot = (std::ptr::from_ref::<Self>(self) as usize + xor_offset) as *mut GameXorSlot;
        // SAFETY: per fn contract.
        let xor_slot_ref = unsafe { &mut *xor_slot };
        // SAFETY: per fn contract.
        unsafe {
            xor_slot_ref.encrypt(skin);
        }
        // Volatile: the game mutates this field concurrently, so a plain `&mut`
        // write can be optimized away by LLVM (which assumes exclusive access).
        // SAFETY: `&raw mut` of a live game-memory field; write is in-bounds.
        unsafe {
            (&raw mut stack.base_skin.skin).write_volatile(skin);
        }
        // SAFETY: per fn contract.
        let handled = unsafe {
            self.check_special_skins(cds_offset, push_fn, dtor_fn, model, skin, special_skins)
        };
        if !handled {
            // SAFETY: per fn contract.
            unsafe {
                stack.update(update_fn, true);
            }
        }
    }
}

#[repr(C)]
pub struct AIHero {
    pub base: AIBaseCommon,
}
impl Deref for AIHero {
    type Target = AIBaseCommon;
    fn deref(&self) -> &AIBaseCommon {
        &self.base
    }
}

#[repr(C)]
pub struct AITurret {
    pub base: AIBaseCommon,
}
impl Deref for AITurret {
    type Target = AIBaseCommon;
    fn deref(&self) -> &AIBaseCommon {
        &self.base
    }
}

#[repr(C)]
pub struct AIMinionClient {
    pub base: AIBaseCommon,
}
impl Deref for AIMinionClient {
    type Target = AIBaseCommon;
    fn deref(&self) -> &AIBaseCommon {
        &self.base
    }
}

impl AIMinionClient {
    /// # Safety
    /// Caller guarantees `self` is live and `fn_addr` is the address of the
    /// game's gold-redirect-target function, matching this signature.
    pub unsafe fn gold_redirect_target(&self, fn_addr: usize) -> *mut AIBaseCommon {
        // SAFETY: per fn contract.
        let func: GoldRedirectFn = unsafe { std::mem::transmute(fn_addr) };
        // SAFETY: per fn contract.
        unsafe { func(std::ptr::from_ref::<Self>(self) as usize) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_slot_encrypt_matches_the_pure_mix_function() {
        let mut slot = GameXorSlot {
            xor_key: 0,
            values_table: [0; 4],
            key_initialized: 0,
            bytes_xor_count: 0,
            bytes_xor_count_8: 0,
            value_index: 0,
        };
        unsafe { slot.encrypt(7) };
        assert_eq!(slot.key_initialized, 1);
        // After one encrypt(), value_index rotated to 1 and slot[1] holds the mix.
        assert_eq!(slot.value_index, 1);
        let expected = crate::crypt::xor_mix(7, slot.xor_key);
        assert_eq!(slot.values_table[1], expected);
    }

    #[test]
    fn xor_slot_is_24_bytes_matching_the_games_live_field() {
        assert_eq!(size_of::<GameXorSlot>(), 24);
    }
}
