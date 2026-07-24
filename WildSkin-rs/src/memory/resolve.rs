//! Two-phase signature resolution: wait for `GameClient` to reach `Running`,
//! then resolve everything else.

use super::offsets::{FieldOffsets, FnAddresses, GlobalPointers, ResolvedOffsets};
use super::scanner;
use super::signatures::{
    CHAMPION_MANAGER_SIG, CHARACTER_DATA_STACK_OFFSET_SIG, GAME_CLIENT_SIG, GOLD_REDIRECT_FN_SIG,
    HERO_LIST_SIG, MINION_LIST_SIG, MSVC_STRING_DTOR_SIG, PLAYER_SIG, PUSH_FN_SIG,
    SKIN_ID_OFFSET_SIG, TRANSLATE_STRING_FN_SIG, TURRET_LIST_SIG, UPDATE_FN_SIG, WINDOW_SIG,
};
use crate::sdk::game_state::{self, GameClient};

const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Blocks until the game is loaded and its `GameClient` reports `Running`.
///
/// # Safety
/// Must run inside the target game process (reads its own module's `.text`).
pub unsafe fn wait_for_game_client() -> (usize, usize) {
    loop {
        let base = winsafe::HINSTANCE::GetModuleHandle(None).map_or(0, |h| h.ptr() as usize);
        if base != 0 {
            // SAFETY: `base` is this process's own module, per the successful
            // `GetModuleHandleW` above.
            let text = unsafe { scanner::text_section(base) };
            // SAFETY: `base`/`text` describe this process's own module.
            if let Some(slot) = unsafe { scanner::resolve(base, text, &GAME_CLIENT_SIG) } {
                // `sub_base: true`, so `slot` is relative to `base`; add it back.
                // SAFETY: `base + slot` is a live match addressing a pointer-sized global.
                let addr = unsafe { *((base + slot) as *const usize) };
                // SAFETY: `addr` was just read from a live global pointer slot.
                if addr != 0 && unsafe { game_state::is_running(addr as *const GameClient) } {
                    return (base, addr);
                }
            }
        }
        std::thread::sleep(RETRY_INTERVAL);
    }
}

/// Resolves the remaining signatures once the game is `Running`.
///
/// # Safety
/// `base` is this process's own module base and `game_client` a live
/// `GameClient` pointer from `wait_for_game_client`.
pub unsafe fn resolve_all(base: usize, game_client: usize) -> Option<ResolvedOffsets> {
    // SAFETY: per fn contract; `base` is this process's own module base.
    let text = unsafe { scanner::text_section(base) };

    // SAFETY: per fn contract; `base`/`text` are this process's module. Same below.
    let player = unsafe { scanner::resolve(base, text, &PLAYER_SIG) }?;
    // SAFETY: as above.
    let hero_list = unsafe { scanner::resolve(base, text, &HERO_LIST_SIG) }?;
    // SAFETY: as above.
    let champion_manager = unsafe { scanner::resolve(base, text, &CHAMPION_MANAGER_SIG) }?;
    // SAFETY: as above.
    let minion_list = unsafe { scanner::resolve(base, text, &MINION_LIST_SIG) }?;
    // SAFETY: as above.
    let turret_list = unsafe { scanner::resolve(base, text, &TURRET_LIST_SIG) }?;
    // SAFETY: as above.
    let window = unsafe { scanner::resolve(base, text, &WINDOW_SIG) }?;
    // SAFETY: as above.
    let character_data_stack =
        unsafe { scanner::resolve(base, text, &CHARACTER_DATA_STACK_OFFSET_SIG) }?;
    // SAFETY: as above.
    let skin_id = unsafe { scanner::resolve(base, text, &SKIN_ID_OFFSET_SIG) }?;

    // Fn sigs are `sub_base: true`, so add `base` back once here for an
    // already-callable absolute address.
    // SAFETY: as above.
    let character_data_stack_push = base + unsafe { scanner::resolve(base, text, &PUSH_FN_SIG) }?;
    // SAFETY: as above.
    let character_data_stack_update =
        base + unsafe { scanner::resolve(base, text, &UPDATE_FN_SIG) }?;
    // SAFETY: as above.
    let translate_string =
        base + unsafe { scanner::resolve(base, text, &TRANSLATE_STRING_FN_SIG) }?;
    // SAFETY: as above.
    let get_gold_redirect_target =
        base + unsafe { scanner::resolve(base, text, &GOLD_REDIRECT_FN_SIG) }?;
    // SAFETY: as above.
    let msvc_string_dtor = base + unsafe { scanner::resolve(base, text, &MSVC_STRING_DTOR_SIG) }?;

    // Globals are pointers-to-pointers in static data, so each needs one deref
    // (`base +` first, since `sub_base: true`).
    // SAFETY: each is a live pointer-sized global from its own sig. Same below.
    let player = unsafe { *((base + player) as *const usize) };
    // SAFETY: as above.
    let champion_manager = unsafe { *((base + champion_manager) as *const usize) };
    // SAFETY: as above.
    let window = unsafe { *((base + window) as *const usize) };
    // SAFETY: as above.
    let hero_list = unsafe { *((base + hero_list) as *const usize) };
    // SAFETY: as above.
    let minion_list = unsafe { *((base + minion_list) as *const usize) };
    // SAFETY: as above.
    let turret_list = unsafe { *((base + turret_list) as *const usize) };

    // Reinterpret each address as its typed `*Fn` once in `from_addrs`.
    // SAFETY: each address is the game's real function, matching its `*Fn` type.
    let fns = unsafe {
        FnAddresses::from_addrs(
            character_data_stack_push,
            character_data_stack_update,
            msvc_string_dtor,
            translate_string,
            get_gold_redirect_target,
        )
    };

    Some(ResolvedOffsets::new(
        GlobalPointers {
            base,
            player,
            champion_manager,
            window,
            hero_list,
            minion_list,
            turret_list,
            game_client,
        },
        FieldOffsets {
            character_data_stack,
            skin_id,
        },
        fns,
    ))
}
