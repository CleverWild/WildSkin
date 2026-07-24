//! The "Local Player" tab: the local champion's skin, gear, and ward combos.

use super::widgets::{footer, skin_combo, string_combo};
use crate::memory::ResolvedOffsets;
use crate::sdk::ai_base_common::AIBaseCommon;
use crate::util::fnv::fnv1a;
use hudhook::imgui::Ui;
use std::ffi::CString;

/// Renders the local player's skin, special-gear (only when the skin has gear
/// variants), and ward combos; applies each selection immediately and persists.
pub(super) fn render_local_player_tab(ui: &Ui, off: &ResolvedOffsets, p_ref: &AIBaseCommon) {
    let state = crate::state::get();
    let mut config = state.config.lock().unwrap();
    // SAFETY: `character_data_stack` is the correct resolved offset.
    let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
    // SAFETY: `model` is a valid MSVC string on a live stack.
    let model = unsafe { stack.base_skin.model.as_str() };
    let (champ_hash, current_skin, live_model) =
        (fnv1a(model), stack.base_skin.skin, model.to_owned());
    ui.text("Player Skins Settings:");

    let empty = Vec::new();
    let values = state
        .database
        .champions_skins
        .get(&champ_hash)
        .unwrap_or(&empty);
    if skin_combo(
        ui,
        "Current Skin",
        &mut config.current_combo_skin_index,
        values,
    ) {
        if config.current_combo_skin_index > 0
            && let Some(entry) = values.get((config.current_combo_skin_index - 1) as usize)
            && let Ok(c_model) = CString::new(entry.model_name.clone())
        {
            // SAFETY: `p_ref` live, `off`'s offsets/addresses correct.
            unsafe {
                p_ref.change_skin(
                    off.fields.character_data_stack,
                    off.fields.skin_id,
                    &off.fns.skin_apply,
                    &c_model,
                    entry.skin_id,
                    &state.database.special_skins,
                );
            }
        }
        config.save(&crate::app::config::config_dir(), Some(&live_model));
    }

    if let Some(special) = state
        .database
        .special_skins
        .iter()
        .find(|s| s.champ_hash == champ_hash)
        && let Some(gears) = special.gear_variants_for(current_skin)
    {
        // SAFETY: `p_ref` live, `character_data_stack` correct.
        let mut gear = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) }
            .base_skin
            .gear as i32;
        if string_combo(ui, "Current Gear", &mut gear, gears, false) {
            // SAFETY: `p_ref` live, `character_data_stack` correct.
            let stack = unsafe { p_ref.character_data_stack_mut(off.fields.character_data_stack) };
            stack.base_skin.gear = gear as i8;
            // SAFETY: `stack` live, update fn address correct.
            unsafe {
                stack.update(off.fns.skin_apply.update, true);
            }
        }
        ui.separator();
    }

    let ward_items: Vec<&str> = state
        .database
        .wards_skins
        .iter()
        .map(|(_, name)| name.as_str())
        .collect();
    if string_combo(
        ui,
        "Current Ward Skin",
        &mut config.current_combo_ward_index,
        &ward_items,
        true,
    ) {
        config.current_ward_skin_index = if config.current_combo_ward_index == 0 {
            -1
        } else {
            state.database.wards_skins[(config.current_combo_ward_index - 1) as usize].0 as i32
        };
    }
    footer(ui);
}
