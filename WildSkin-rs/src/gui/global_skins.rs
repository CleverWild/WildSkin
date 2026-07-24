//! The "Global Skins" tab: minion, turret, and jungle-mob skin selection.

use super::widgets::{footer, string_combo};
use crate::memory::ResolvedOffsets;
use crate::sdk::ai_base_common::AIBaseCommon;
use hudhook::imgui::Ui;

/// Applies `skin_id` to every turret on `team`. Skins are ally/enemy pairs
/// (`skin_id*2` vs `skin_id*2+1`); `player_team` picks which half.
fn change_turret_skin(off: &ResolvedOffsets, skin_id: i32, team: i8, player_team: i8) {
    if skin_id == -1 {
        return;
    }
    // SAFETY: called only while the game is `Running`, so the turret list is live.
    let turrets = unsafe { off.turret_list() };
    for &turret_ref in turrets {
        if turret_ref.team == team {
            let final_skin = if player_team == team {
                skin_id * 2
            } else {
                skin_id * 2 + 1
            };
            // SAFETY: `turret_ref` live, `character_data_stack` correct.
            let stack =
                unsafe { turret_ref.character_data_stack_mut(off.fields.character_data_stack) };
            stack.base_skin.skin = final_skin;
            // SAFETY: `stack` live, update fn address correct.
            unsafe {
                stack.update(off.fns.skin_apply.update, true);
            }
        }
    }
}

/// Renders the minion/turret/jungle-mob combos. Minion/jungle-mob just update
/// config (`apply_frame` reapplies each frame since they respawn); turrets
/// apply immediately since they don't respawn.
pub(super) fn render_global_skins_tab(
    ui: &Ui,
    off: &ResolvedOffsets,
    player: Option<&AIBaseCommon>,
) {
    let state = crate::state::get();
    let mut config = state.config.lock().unwrap();
    ui.text("Global Skins Settings:");

    if string_combo(
        ui,
        "Minion Skins:",
        &mut config.current_combo_minion_index,
        &state.database.minions_skins,
        true,
    ) {
        config.current_minion_skin_index = config.current_combo_minion_index - 1;
    }
    ui.separator();

    let player_team = player.map_or(1, |p_ref| p_ref.team);
    if string_combo(
        ui,
        "Order Turret Skins:",
        &mut config.current_combo_order_turret_index,
        &state.database.turret_skins,
        true,
    ) {
        let idx = config.current_combo_order_turret_index;
        let skin = if idx >= 17 { idx + 1 } else { idx - 1 };
        change_turret_skin(off, skin, 1, player_team);
    }
    if string_combo(
        ui,
        "Chaos Turret Skins:",
        &mut config.current_combo_chaos_turret_index,
        &state.database.turret_skins,
        true,
    ) {
        let idx = config.current_combo_chaos_turret_index;
        let skin = if idx >= 17 { idx + 1 } else { idx - 1 };
        change_turret_skin(off, skin, 2, player_team);
    }
    ui.separator();

    ui.text("Jungle Mobs Skins Settings:");
    for mob in &state.database.jungle_mobs_skins {
        let label = format!("Current {} skin", mob.name);
        let first_hash = mob.name_hashes[0];
        let entry_idx = config
            .current_combo_jungle_mob_skin_index
            .entry(first_hash)
            .or_insert(0);
        let mut idx = *entry_idx;
        if string_combo(ui, &label, &mut idx, &mob.skins, true) {
            for &hash in &mob.name_hashes {
                config.current_combo_jungle_mob_skin_index.insert(hash, idx);
            }
        }
    }
    drop(config);
    footer(ui);
}
