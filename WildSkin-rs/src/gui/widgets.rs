//! Shared imgui widgets used across the menu tabs: the two skin combo boxes,
//! the centered footer, and the click-to-rebind hotkey widget.

use crate::keybind::KeyBind;
use crate::skin_database::SkinInfo;
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

/// Combo box over a plain string list (skin names come pre-formatted from
/// `SkinDatabase` for minions/turrets/jungle mobs, unlike `skin_combo`'s
/// `SkinInfo` slice). `with_default_prefix` controls whether a "Default"
/// entry is prepended at index 0. Returns `true` the frame the selection
/// changes.
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
    // NOTE: original shows compiler-provided __DATE__/__TIME__; Rust has no
    // direct equivalent without a build.rs timestamp — this substitutes the
    // crate version, a deliberate, smaller change flagged here rather than
    // adding a build-script dependency just to reproduce a build timestamp
    // string with no functional effect on the tool itself.
    let text_width = ui.calc_text_size(&build_text)[0];
    ui.set_cursor_pos([(ui.window_size()[0] - text_width) / 2.0, ui.cursor_pos()[1]]);
    ui.text(&build_text);
    let copyright = "Copyright (C) 2026 CleverWild";
    let cw = ui.calc_text_size(copyright)[0];
    ui.set_cursor_pos([(ui.window_size()[0] - cw) / 2.0, ui.cursor_pos()[1]]);
    ui.text(copyright);
}

/// Tracks which hotkey widget (if any) is currently "armed" waiting for a
/// keypress. The original does this via `ImGui`'s own active-widget-id
/// machinery (`GetActiveID`/`SetActiveID`/`ClearActiveID`); imgui-rs doesn't
/// cleanly expose those internals, so this port uses a small Rust-side
/// capture flag instead — same observable behavior (click to arm, press a
/// key to bind, click again to cancel), without depending on raw `ImGui`
/// internals Step 4 would otherwise have to chase down.
static CAPTURING_HOTKEY: std::sync::Mutex<Option<&'static str>> = std::sync::Mutex::new(None);

/// Click-to-rebind hotkey button: shows the bound key's name, and on click
/// arms capture mode (see `CAPTURING_HOTKEY`) until the next keypress binds
/// `key` or the button is clicked again to cancel.
pub(super) fn hotkey_widget(ui: &Ui, label: &'static str, key: &mut KeyBind) {
    ui.text(label);
    ui.same_line();
    let mut capturing = CAPTURING_HOTKEY.lock().unwrap();
    if *capturing == Some(label) {
        // Short-circuits exactly like the previous if/else-if: if the
        // cancel button is clicked this frame, `set_to_pressed_key` (which
        // has the side effect of binding `key`) is never called, same as
        // before.
        if ui.button_with_size("...", [100.0, 0.0]) || key.set_to_pressed_key(ui) {
            *capturing = None;
        }
    } else if ui.button_with_size(key.to_string(), [100.0, 0.0]) {
        *capturing = Some(label);
    }
}
