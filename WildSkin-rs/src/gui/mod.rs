//! Ports `GUI.cpp` tab-for-tab: the whole imgui menu, split one file per tab.
//!
//! - [`widgets`] — shared combo/footer/hotkey widgets.
//! - [`keybind_input`] — imgui-driven `KeyBind` press/hold/capture logic.
//! - `local_player` / `other_champs` / `global_skins` / `logger_tab` /
//!   `extras` — one module per menu tab.
//!
//! Every raw-memory touch here routes through already-reviewed earlier tasks
//! (`sdk::*`, `skin_database`, `config`, `state`) — this surface is imgui
//! widget wiring and pure logic, not new unsafe/FFI surface.

mod extras;
mod global_skins;
mod keybind_input;
mod local_player;
mod logger_tab;
mod other_champs;
mod widgets;

use hudhook::imgui::{self, Ui};

/// Draws the main menu window and dispatches to each tab module. Called once
/// per frame from `Overlay::render` while the menu is open.
pub fn render(ui: &Ui) {
    let state = crate::state::get();
    let off = &state.offsets;
    let player = off.player();
    // SAFETY: `render` is only called (via Task 18's `Overlay::render`)
    // while the game is `Running` and `state::get()` is initialized, so the
    // player, if present, is live.
    let player_ref = unsafe { off.player_ref() };
    // SAFETY: `render` is only called (via Task 18's `Overlay::render`)
    // while the game is `Running` and `state::get()` is initialized, so the
    // hero list is live.
    let heroes = unsafe { off.hero_list() };
    let my_team = player_ref.map_or(1, |p_ref| {
        // SAFETY: `p_ref` is live, per `off.player_ref()`'s own contract.
        unsafe { p_ref.team() }
    });

    ui.window(shared::APP_NAME)
        .flags(
            imgui::WindowFlags::NO_COLLAPSE
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::ALWAYS_AUTO_RESIZE,
        )
        .build(|| {
            if let Some(_tab_guard) = ui.tab_bar("TabBar") {
                if let Some(p_ref) = player_ref
                    && let Some(_tab_guard) = ui.tab_item("Local Player")
                {
                    local_player::render_local_player_tab(ui, off, p_ref);
                }

                if heroes.len() > 1
                    && let Some(_tab_guard) = ui.tab_item("Other Champs")
                {
                    other_champs::render_other_champs_tab(ui, off, heroes, player, my_team);
                }

                if let Some(_tab_guard) = ui.tab_item("Global Skins") {
                    global_skins::render_global_skins_tab(ui, off, player_ref);
                }

                if let Some(_tab_guard) = ui.tab_item("Logger") {
                    logger_tab::render_logger_tab(ui);
                }

                if let Some(_tab_guard) = ui.tab_item("Extras") {
                    extras::render_extras_tab(ui, off, player, heroes, my_team);
                }
            }
        });
}
