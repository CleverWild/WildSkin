//! The raw-key half of the original's `wndProc`: menu toggle, Ctrl+5 gear
//! cycling, and the quick-skin-change previous/next hotkeys.

use crate::fnv::fnv1a;
use std::ffi::CString;

const VK_5: i32 = 0x35; // literal '5' key — Ctrl+5 cycles "special" gear variants, NOT F5 (0x74)

/// Ports the raw-key-handling half of the original's `wndProc` (the part
/// that runs regardless of whether `ImGui` captured the keystroke). The
/// original's F7 handler (`testFunc`) is a developer debug scratch hook,
/// explicitly commented in the source as an example — dropped here, not
/// translated.
///
/// # Safety
/// Caller must be running inside the target game process with `state::get()`
/// already initialized and the offsets it holds resolved against a live,
/// `Running` game.
pub unsafe fn handle_keydown(vk_code: i32) {
    let state = crate::state::get();
    let off = &state.offsets;

    let menu_key = state.config.lock().unwrap().menu_key.vk_code();
    if vk_code == menu_key {
        let now_open = state.toggle_menu_open();
        if !now_open {
            // SAFETY: caller guarantees the player, if present, is live.
            let model = unsafe { off.player_ref() }.map(|p_ref| {
                // SAFETY: `off.fields.character_data_stack` was resolved against
                // this same live process.
                let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: `model` is a valid MSVC string on a live stack.
                unsafe { stack.base_skin.model.as_str() }.to_owned()
            });
            state.config.lock().unwrap().save(&crate::config::config_dir(), model.as_deref());
        }
        return;
    }

    if vk_code == VK_5 {
        let ctrl_down = winsafe::GetAsyncKeyState(winsafe::co::VK::LCONTROL);
        // SAFETY: caller guarantees the player, if present, is live.
        if let (true, Some(p_ref)) = (ctrl_down, unsafe { off.player_ref() }) {
            // SAFETY: `off.fields.character_data_stack` was resolved against this
            // same live process.
            let stack = unsafe { p_ref.character_data_stack_mut(off.fields.character_data_stack) };
            // SAFETY: `model` is a valid MSVC string on a live stack.
            let champ_hash = fnv1a(unsafe { stack.base_skin.model.as_str() });
            let skin = stack.base_skin.skin;
            if let Some(special) = state.database.special_skins.iter().find(|s| s.champ_hash == champ_hash)
                && let Some(gears) = special.gear_variants_for(skin)
            {
                let max_gear = gears.len() as i8 - 1;
                if stack.base_skin.gear < max_gear {
                    stack.base_skin.gear += 1;
                } else {
                    stack.base_skin.gear = 0;
                }
                // SAFETY: caller guarantees `stack` is live.
                unsafe { stack.update(off.fns.character_data_stack_update, true); }
            }
        }
        return;
    }

    let quick_skin_change = state.config.lock().unwrap().quick_skin_change;
    if !quick_skin_change {
        return; // F7 debug scratch hook intentionally dropped — see module doc.
    }

    let next_key = state.config.lock().unwrap().next_skin_key.vk_code();
    let previous_key = state.config.lock().unwrap().previous_skin_key.vk_code();

    if vk_code == next_key {
        // SAFETY: caller guarantees the player, if present, is live.
        if let Some(p_ref) = unsafe { off.player_ref() } {
            let model = {
                // SAFETY: `off.fields.character_data_stack` was resolved against
                // this same live process.
                let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: `model` is a valid MSVC string on a live stack.
                unsafe { stack.base_skin.model.as_str() }.to_owned()
            };
            let hash = fnv1a(&model);
            let empty = Vec::new();
            let values = state.database.champions_skins.get(&hash).unwrap_or(&empty);

            let mut config = state.config.lock().unwrap();
            config.current_combo_skin_index += 1;
            let max = values.len() as i32;
            if config.current_combo_skin_index > max {
                config.current_combo_skin_index = max;
            }
            if config.current_combo_skin_index > 0
                && let Some(entry) = values.get((config.current_combo_skin_index - 1) as usize)
                    && let Ok(c_model) = CString::new(entry.model_name.clone()) {
                        // SAFETY: `off`'s offsets/function addresses are
                        // correct for `p_ref`.
                        unsafe {
                            p_ref.change_skin(off.fields.character_data_stack, off.fields.skin_id, off.fns.character_data_stack_push, off.fns.msvc_string_dtor, &c_model, entry.skin_id, &state.database.special_skins);
                        }
                    }
            drop(config);
            state.config.lock().unwrap().save(&crate::config::config_dir(), Some(&model));
        }
    } else if vk_code == previous_key
        // SAFETY: caller guarantees the player, if present, is live.
        && let Some(p_ref) = unsafe { off.player_ref() } {
            let model = {
                // SAFETY: `off.fields.character_data_stack` was resolved against
                // this same live process.
                let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: `model` is a valid MSVC string on a live stack.
                unsafe { stack.base_skin.model.as_str() }.to_owned()
            };
            let hash = fnv1a(&model);
            let empty = Vec::new();
            let values = state.database.champions_skins.get(&hash).unwrap_or(&empty);

            let mut config = state.config.lock().unwrap();
            config.current_combo_skin_index -= 1;
            if config.current_combo_skin_index > 0 {
                if let Some(entry) = values.get((config.current_combo_skin_index - 1) as usize)
                    && let Ok(c_model) = CString::new(entry.model_name.clone()) {
                        // SAFETY: `off`'s offsets/function addresses are
                        // correct for `p_ref`.
                        unsafe {
                            p_ref.change_skin(off.fields.character_data_stack, off.fields.skin_id, off.fns.character_data_stack_push, off.fns.msvc_string_dtor, &c_model, entry.skin_id, &state.database.special_skins);
                        }
                    }
            } else {
                config.current_combo_skin_index = 1;
            }
            drop(config);
            state.config.lock().unwrap().save(&crate::config::config_dir(), Some(&model));
        }
}
