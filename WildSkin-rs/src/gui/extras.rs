//! The "Extras" tab: hotkey rebinds, toggles, nick change, bulk skin actions,
//! font scale, and the force-close button.

use super::widgets::{footer, hotkey_widget};
use crate::memory::ResolvedOffsets;
use crate::sdk::ai_base_common::{AIBaseCommon, AIHero};
use crate::util::fnv::fnv1a;
use hudhook::imgui::Ui;
use std::ffi::CString;

/// Renders the Extras tab: rebinds, toggles, nick field, bulk skin buttons,
/// font-scale slider, FPS, and the unhook-and-exit "Force Close" button.
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
        // SAFETY: `p` is live; `AIBaseCommon` is `#[repr(C)]` with `GameObject`
        // first, so the cast reaches `name` (only `Deref`, not `DerefMut`, so
        // autoderef can't).
        let game_object = unsafe { &mut *p.cast::<crate::sdk::game_object::GameObject>() };
        let name = &mut game_object.name;
        // SAFETY: `name` is a live, initialized MSVC string.
        let mut buf = unsafe { name.as_str() }.to_owned();
        if ui.input_text("Change Nick", &mut buf).build() {
            name.set_sso(&buf);
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
                // SAFETY: `hero_ref` live, `character_data_stack` correct.
                let stack =
                    unsafe { hero_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: `model` is a valid MSVC string on a live stack.
                let model = unsafe { stack.base_skin.model.as_str() }.to_owned();
                if let Ok(c_model) = CString::new(model) {
                    // SAFETY: `hero_ref` live, `off`'s offsets/addresses correct.
                    unsafe {
                        hero_ref.change_skin(
                            off.fields.character_data_stack,
                            off.fields.skin_id,
                            &off.fns.skin_apply,
                            &c_model,
                            0,
                            &state.database.special_skins,
                        );
                    }
                }
            }
        }
        config.save(&crate::app::config::config_dir(), None);
    }

    if ui.button("Random Skins") {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
        let mut rand_range = |max: usize| -> usize {
            // ponytail: xorshift, not a crate; single call site needing an
            // index, not a general RNG.
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            1 + (seed as usize % max)
        };
        for &hero_ref in heroes {
            // SAFETY: `hero_ref` live, `character_data_stack` correct.
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
                let hero_team = hero_ref.team;
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
                // SAFETY: `hero_ref` live, `off`'s offsets/addresses correct.
                unsafe {
                    hero_ref.change_skin(
                        off.fields.character_data_stack,
                        off.fields.skin_id,
                        &off.fns.skin_apply,
                        &c_model,
                        entry.skin_id,
                        &state.database.special_skins,
                    );
                }
            }
        }
        config.save(&crate::app::config::config_dir(), None);
    }

    ui.slider("Font Scale", 1.0, 2.0, &mut config.font_scale);
    drop(config);

    if ui.button("Force Close") {
        // hudhook owns the hook lifetime, so we can eject and exit directly
        // from this callback (the original needs a separate polling loop).
        hudhook::eject();
        #[allow(
            clippy::exit,
            reason = "deliberate user-triggered process exit, matches the original's ExitProcess(0)"
        )]
        std::process::exit(0);
    }
    ui.text(format!("FPS: {:.0} FPS", ui.io().framerate));
    footer(ui);
}
