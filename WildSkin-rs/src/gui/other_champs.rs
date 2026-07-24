//! The "Other Champs" tab: per-champion skin combos, grouped by ally/enemy.

use super::widgets::{footer, skin_combo};
use crate::memory::ResolvedOffsets;
use crate::sdk::ai_base_common::{AIBaseCommon, AIHero};
use crate::util::fnv::fnv1a;
use hudhook::imgui::Ui;
use std::ffi::CString;

/// Per-hero skin combo for every champion but the local player, grouped by
/// team under an Ally/Enemy separator. Applies on select and persists the maps.
pub(super) fn render_other_champs_tab(
    ui: &Ui,
    off: &ResolvedOffsets,
    heroes: &[&AIHero],
    player: Option<*mut AIBaseCommon>,
    my_team: i8,
) {
    let state = crate::state::get();
    ui.text("Other Champs Skins Settings:");
    let mut last_team: i8 = 0;

    for &hero_ref in heroes {
        let hero_addr = std::ptr::from_ref(hero_ref) as usize;
        if player.map(|p| p as usize) == Some(hero_addr) {
            continue;
        }
        // SAFETY: `hero_ref` live, `character_data_stack` correct.
        let stack = unsafe { hero_ref.character_data_stack_ref(off.fields.character_data_stack) };
        // SAFETY: `model` is a valid MSVC string on a live stack.
        let model = unsafe { stack.base_skin.model.as_str() };
        let (champ_hash, model_name) = (fnv1a(model), model.to_owned());
        if champ_hash == fnv1a("PracticeTool_TargetDummy") {
            continue;
        }

        let hero_team = hero_ref.team;
        let is_enemy = hero_team != my_team;
        if last_team == 0 || hero_team != last_team {
            if last_team != 0 {
                ui.separator();
            }
            ui.text(if is_enemy {
                " Enemy champions"
            } else {
                " Ally champions"
            });
            last_team = hero_team;
        }

        let mut config = state.config.lock().unwrap();

        let name = &hero_ref.name;
        // SAFETY: `name` was just resolved above from a live `hero_ref`.
        let hero_name = unsafe { name.as_str() }.to_owned();
        let label = if config.hero_name {
            format!("HeroName: [ {model_name} ]##{hero_addr:X}")
        } else {
            format!("PlayerName: [ {hero_name} ]##{hero_addr:X}")
        };

        let map = if is_enemy {
            &mut config.current_combo_enemy_skin_index
        } else {
            &mut config.current_combo_ally_skin_index
        };
        let entry_idx = map.entry(champ_hash).or_insert(0);

        let empty = Vec::new();
        let values = state
            .database
            .champions_skins
            .get(&champ_hash)
            .unwrap_or(&empty);
        let mut idx = *entry_idx;
        if skin_combo(ui, &label, &mut idx, values) {
            *entry_idx = idx;
            if idx > 0
                && let Some(skin) = values.get((idx - 1) as usize)
                && let Ok(c_model) = CString::new(skin.model_name.clone())
            {
                // SAFETY: `hero_ref` live, `off`'s offsets/addresses correct.
                unsafe {
                    hero_ref.change_skin(
                        off.fields.character_data_stack,
                        off.fields.skin_id,
                        &off.fns.skin_apply,
                        &c_model,
                        skin.skin_id,
                        &state.database.special_skins,
                    );
                }
            }
            drop(config);
            state
                .config
                .lock()
                .unwrap()
                .save(&crate::app::config::config_dir(), None);
        }
    }
    footer(ui);
}
