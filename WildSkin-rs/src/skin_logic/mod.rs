//! Skin-application logic and the raw key-handling half of `wndProc`.
//! Highest-traffic area (`apply_frame` runs every frame) and untestable here:
//! both call through `unsafe` FFI into live game structs this sandbox can't
//! fabricate, so correctness is verified in-game. Kept a literal transcription.
//!
//! - [`apply_frame`] per-frame re-sync + one-shot saved-choice apply.
//! - [`handle_keydown`] the raw-key half of `wndProc`.

mod apply_frame;
mod handle_keydown;

pub use apply_frame::apply_frame;
pub use handle_keydown::handle_keydown;
