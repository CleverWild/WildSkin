//! imgui-driven hotkey input for `KeyBind`: per-frame press/hold checks and
//! the click-to-rebind capture scan.

use crate::app::keybind::{self, KeyBind, KeyCode};
use hudhook::imgui::{self, Ui};

// Per-frame hotkey checks for the quick-skin-change feature. The polling that
// calls these isn't wired into the render loop yet, so they read as dead.
impl KeyBind {
    /// `true` on the single frame `self`'s bound key/mouse-button/wheel
    /// direction transitions to pressed (no auto-repeat).
    #[expect(
        dead_code,
        reason = "per-frame hotkey check for the not-yet-wired quick-skin-change feature, see comment above"
    )]
    pub fn is_pressed(&self, ui: &Ui) -> bool {
        if !self.is_set() {
            return false;
        }
        if self.code() == KeyCode::MousewheelDown {
            return ui.io().mouse_wheel < 0.0;
        }
        if self.code() == KeyCode::MousewheelUp {
            return ui.io().mouse_wheel > 0.0;
        }
        if is_mouse_code(self.code()) {
            let button = self.code() as i32 - KeyCode::Mouse1 as i32;
            return ui.is_mouse_clicked(mouse_button_from_index(button));
        }
        ui.is_key_index_pressed_no_repeat(self.vk_code() as u32)
    }

    /// `true` on every frame `self`'s bound key/mouse-button is held down
    /// (unlike `is_pressed`, repeats every frame, not just the transition).
    #[expect(
        dead_code,
        reason = "per-frame hotkey check for the not-yet-wired quick-skin-change feature, see comment above"
    )]
    pub fn is_down(&self, ui: &Ui) -> bool {
        if !self.is_set() {
            return false;
        }
        if self.code() == KeyCode::MousewheelDown {
            return ui.io().mouse_wheel < 0.0;
        }
        if self.code() == KeyCode::MousewheelUp {
            return ui.io().mouse_wheel > 0.0;
        }
        if is_mouse_code(self.code()) {
            let button = self.code() as i32 - KeyCode::Mouse1 as i32;
            return ui.is_mouse_down(mouse_button_from_index(button));
        }
        ui.is_key_index_down(self.vk_code() as u32)
    }

    /// Scans for the first pressed key/mouse input and binds `self` to it.
    /// Escape clears; wheel and five mouse buttons are checked before the VK
    /// range; LCTRL is promoted to RALT when RALT is also down (`AltGr` reports
    /// as a phantom LCTRL+RALT pair). Returns `true` the frame one is captured.
    pub fn set_to_pressed_key(&mut self, ui: &Ui) -> bool {
        const VK_ESCAPE: u32 = 0x1B;
        if ui.is_key_index_pressed_no_repeat(VK_ESCAPE) {
            *self = Self::new(KeyCode::None);
            return true;
        }

        let wheel = ui.io().mouse_wheel;
        if wheel < 0.0 {
            *self = Self::new(KeyCode::MousewheelDown);
            return true;
        }
        if wheel > 0.0 {
            *self = Self::new(KeyCode::MousewheelUp);
            return true;
        }

        for i in 0..5 {
            if ui.is_mouse_clicked(mouse_button_from_index(i)) {
                let code: KeyCode =
                    // SAFETY: `KeyCode` is `#[repr(u8)]` with `Mouse1..=Mouse5`
                    // consecutive, so `Mouse1 as u8 + i` (i in 0..5) is in range.
                    unsafe { std::mem::transmute(KeyCode::Mouse1 as u8 + i as u8) };
                *self = Self::new(code);
                return true;
            }
        }

        for vk in 0..256i32 {
            if !ui.is_key_index_pressed_no_repeat(vk as u32) {
                continue;
            }
            if let Some(mut code) = keybind::vk_to_code(vk) {
                if code == KeyCode::Lctrl {
                    let ralt = Self::new(KeyCode::Ralt);
                    if ui.is_key_index_pressed_no_repeat(ralt.vk_code() as u32) {
                        code = KeyCode::Ralt;
                    }
                }
                *self = Self::new(code);
                return true;
            }
        }
        false
    }
}

/// `KeyCode` isn't `PartialOrd`, so compare discriminants directly rather than
/// `RangeInclusive::contains` on the enum.
#[allow(
    dead_code,
    reason = "helper for the not-yet-wired is_pressed/is_down per-frame hotkey checks above"
)]
fn is_mouse_code(code: KeyCode) -> bool {
    let c = code as u8;
    (KeyCode::Mouse1 as u8..=KeyCode::Mouse5 as u8).contains(&c)
}

/// `imgui::MouseButton` (0.12) has no by-index constructor, so hand-match the
/// `0..5` index range to its variants.
const fn mouse_button_from_index(i: i32) -> imgui::MouseButton {
    match i {
        0 => imgui::MouseButton::Left,
        1 => imgui::MouseButton::Right,
        2 => imgui::MouseButton::Middle,
        3 => imgui::MouseButton::Extra1,
        _ => imgui::MouseButton::Extra2,
    }
}
