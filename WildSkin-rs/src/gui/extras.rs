//! The "Extras" tab: hotkey rebinds, toggles, nick change, bulk skin actions,
//! font scale, and the force-close button.

use super::widgets::{footer, hotkey_widget};
use crate::fnv::fnv1a;
use crate::memory::ResolvedOffsets;
use crate::sdk::ai_base_common::{AIBaseCommon, AIHero};
use hudhook::imgui::Ui;
use std::ffi::CString;

/// Renders hotkey rebinds, misc toggles, the nickname field, the two bulk
/// skin-change buttons ("No skins except local player" / "Random Skins"),
/// the font-scale slider, FPS counter, and the "Force Close" button that
/// unhooks and exits the process.
pub(super) fn render_extras_tab(
    ui: &Ui,
    off: &ResolvedOffsets,
    player: Option<*mut AIBaseCommon>,
    heroes: &[&AIHero],
    my_team: i8,
) {
    let state = crate::state::get();
    let mut config = state.config.lock().unwrap();

    hotkey_widget(ui, "Menu Key", &mut config.menu_key);
    ui.checkbox("Auto Show Menu", &mut config.is_open);
    ui.checkbox(
        if config.hero_name {
            "HeroName based"
        } else {
            "PlayerName based"
        },
        &mut config.hero_name,
    );
    ui.checkbox("Rainbow Text", &mut config.rainbow_text);
    ui.checkbox("Quick Skin Change", &mut config.quick_skin_change);

    if config.quick_skin_change {
        ui.separator();
        hotkey_widget(ui, "Previous Skin Key", &mut config.previous_skin_key);
        hotkey_widget(ui, "Next Skin Key", &mut config.next_skin_key);
        ui.separator();
    }

    if let Some(p) = player {
        // SAFETY: `p` is a live player pointer; `AIBaseCommon` is `#[repr(C)]`
        // with `GameObject` as its first field, so a raw `*mut AIBaseCommon`
        // may be reinterpreted as `*mut GameObject` to reach `name_mut`
        // (`AIBaseCommon` only implements `Deref`, not `DerefMut`, so
        // autoderef can't reach it directly).
        let game_object = unsafe { &mut *p.cast::<crate::sdk::game_object::GameObject>() };
        // SAFETY: `game_object` was just derived above from a live pointer.
        let name = unsafe { game_object.name_mut() };
        // SAFETY: `name` is a live, initialized MSVC string.
        let mut buf = unsafe { name.as_str() }.to_owned();
        if ui.input_text("Change Nick", &mut buf).build() {
            // SAFETY: `name` is a live, initialized MSVC string.
            unsafe {
                name.set_sso(&buf);
            }
        }
    }

    if ui.button("No skins except local player") {
        for v in config.current_combo_enemy_skin_index.values_mut() {
            *v = 1;
        }
        for v in config.current_combo_ally_skin_index.values_mut() {
            *v = 1;
        }
        for &hero_ref in heroes {
            if player.map(|p| p as usize) != Some(std::ptr::from_ref(hero_ref) as usize) {
                // SAFETY: `hero_ref` is live and `character_data_stack` is the
                // correct offset for it.
                let stack =
                    unsafe { hero_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: `model` is a valid MSVC string on a live stack.
                let model = unsafe { stack.base_skin.model.as_str() }.to_owned();
                if let Ok(c_model) = CString::new(model) {
                    // SAFETY: `hero_ref` is live and `off`'s offsets/function
                    // addresses are correct for it.
                    unsafe {
                        hero_ref.change_skin(
                            off.fields.character_data_stack,
                            off.fields.skin_id,
                            off.fns.character_data_stack_push,
                            off.fns.msvc_string_dtor,
                            &c_model,
                            0,
                            &state.database.special_skins,
                        );
                    }
                }
            }
        }
        config.save(&crate::config::config_dir(), None);
    }

    if ui.button("Random Skins") {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
        let mut rand_range = |max: usize| -> usize {
            // ponytail: xorshift, not a crate — this is a single call site
            // needing "pick an index," not a general RNG facility.
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            1 + (seed as usize % max)
        };
        for &hero_ref in heroes {
            // SAFETY: `hero_ref` is live and `character_data_stack` is the
            // correct offset for it.
            let stack =
                unsafe { hero_ref.character_data_stack_ref(off.fields.character_data_stack) };
            // SAFETY: `model` is a valid MSVC string on a live stack.
            let champ_hash = fnv1a(unsafe { stack.base_skin.model.as_str() });
            if champ_hash == fnv1a("PracticeTool_TargetDummy") {
                continue;
            }
            let empty = Vec::new();
            let values = state
                .database
                .champions_skins
                .get(&champ_hash)
                .unwrap_or(&empty);
            if values.is_empty() {
                continue;
            }
            let picked = rand_range(values.len());
            let is_local =
                player.map(|p| p as usize) == Some(std::ptr::from_ref(hero_ref) as usize);
            if is_local {
                config.current_combo_skin_index = picked as i32;
            } else {
                // SAFETY: `hero_ref` is live.
                let hero_team = unsafe { hero_ref.team() };
                let map = if hero_team == my_team {
                    &mut config.current_combo_ally_skin_index
                } else {
                    &mut config.current_combo_enemy_skin_index
                };
                map.insert(champ_hash, picked as i32);
            }
            if let Some(entry) = values.get(picked - 1)
                && let Ok(c_model) = CString::new(entry.model_name.clone())
            {
                // SAFETY: `hero_ref` is live and `off`'s offsets/function
                // addresses are correct for it.
                unsafe {
                    hero_ref.change_skin(
                        off.fields.character_data_stack,
                        off.fields.skin_id,
                        off.fns.character_data_stack_push,
                        off.fns.msvc_string_dtor,
                        &c_model,
                        entry.skin_id,
                        &state.database.special_skins,
                    );
                }
            }
        }
        config.save(&crate::config::config_dir(), None);
    }

    ui.slider("Font Scale", 1.0, 2.0, &mut config.font_scale);
    drop(config);

    if ui.button("Force Close") {
        // Ports Hooks::uninstall(). The original needs a separate polling loop
        // in DllAttach to notice skin-changerState flipping false and call
        // ExitProcess from outside the hook callback; hudhook owns the hook's
        // lifetime here instead, so unhooking and exiting can happen directly
        // from this callback (see Task 19's note on why the original's
        // keep-alive loop isn't ported).
        hudhook::eject();
        // clippy::exit exists to catch accidental early-exits in library code;
        // this is a deliberate, user-triggered process termination in a
        // standalone injected DLL, mirroring the original's own ExitProcess(0)
        // call from its "Force Close" handler.
        #[allow(
            clippy::exit,
            reason = "deliberate user-triggered process exit, matches the original's ExitProcess(0)"
        )]
        std::process::exit(0);
    }
    ui.text(format!("FPS: {:.0} FPS", ui.io().framerate));
    footer(ui);
}
