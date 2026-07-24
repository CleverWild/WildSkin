//! Shared imgui widgets used across the menu tabs: the two skin combo boxes,
//! the centered footer, and the click-to-rebind hotkey widget.

use crate::app::keybind::KeyBind;
use crate::app::skin_database::SkinInfo;
use hudhook::imgui::Ui;

/// Combo box over a champion's skin list, with a leading "Default" entry at
/// index 0. Returns `true` the frame the selection changes.
pub(super) fn skin_combo(ui: &Ui, label: &str, current: &mut i32, values: &[SkinInfo]) -> bool {
    let mut items: Vec<&str> = Vec::with_capacity(values.len() + 1);
    items.push("Default");
    for v in values {
        items.push(v.skin_name.as_str());
    }
    let mut idx = (*current).max(0) as usize;
    let changed = ui.combo(label, &mut idx, &items, |s| std::borrow::Cow::from(*s));
    if changed {
        *current = idx as i32;
    }
    changed
}

/// Combo box over a plain string list (pre-formatted names, unlike
/// `skin_combo`'s `SkinInfo` slice). `with_default_prefix` prepends a "Default"
/// entry at index 0. Returns `true` the frame the selection changes.
pub(super) fn string_combo(
    ui: &Ui,
    label: &str,
    current: &mut i32,
    items: &[&str],
    with_default_prefix: bool,
) -> bool {
    let display: Vec<&str> = if with_default_prefix {
        std::iter::once("Default")
            .chain(items.iter().copied())
            .collect()
    } else {
        items.to_vec()
    };
    let mut idx = (*current).max(0) as usize;
    let changed = ui.combo(label, &mut idx, &display, |s| std::borrow::Cow::from(*s));
    if changed {
        *current = idx as i32;
    }
    changed
}

/// Centered build-version/copyright footer, drawn at the bottom of every tab.
pub(super) fn footer(ui: &Ui) {
    ui.separator();
    let build_text = format!("Last Build: {} - {}", env!("CARGO_PKG_VERSION"), "");
    // NOTE: original shows __DATE__/__TIME__; Rust has no equivalent without a
    // build.rs timestamp, so substitute the crate version.
    let text_width = ui.calc_text_size(&build_text)[0];
    ui.set_cursor_pos([(ui.window_size()[0] - text_width) / 2.0, ui.cursor_pos()[1]]);
    ui.text(&build_text);
    let copyright = "Copyright (C) 2026 CleverWild";
    let cw = ui.calc_text_size(copyright)[0];
    ui.set_cursor_pos([(ui.window_size()[0] - cw) / 2.0, ui.cursor_pos()[1]]);
    ui.text(copyright);
}

/// Which hotkey widget is currently armed for a keypress. imgui-rs doesn't
/// expose `ImGui`'s active-widget-id internals, so use a Rust-side capture flag:
/// same behavior (click to arm, press to bind, click to cancel).
static CAPTURING_HOTKEY: std::sync::Mutex<Option<&'static str>> = std::sync::Mutex::new(None);

/// Click-to-rebind hotkey button: shows the bound key, arms capture on click
/// (see `CAPTURING_HOTKEY`) until a keypress binds `key` or a click cancels.
pub(super) fn hotkey_widget(ui: &Ui, label: &'static str, key: &mut KeyBind) {
    ui.text(label);
    ui.same_line();
    let mut capturing = CAPTURING_HOTKEY.lock().unwrap();
    if *capturing == Some(label) {
        // `||` short-circuits: if the cancel button is clicked, the binding
        // side effect of `set_to_pressed_key` is skipped.
        if ui.button_with_size("...", [100.0, 0.0]) || key.set_to_pressed_key(ui) {
            *capturing = None;
        }
    } else if ui.button_with_size(key.to_string(), [100.0, 0.0]) {
        *capturing = Some(label);
    }
}
