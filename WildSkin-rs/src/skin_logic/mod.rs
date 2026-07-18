//! Ports `Hooks::init()`'s skin-application logic and the raw key-handling
//! half of the original's `wndProc`. This is the highest-traffic area in the
//! port (`apply_frame` runs every rendered frame) and has no unit tests of its
//! own: both functions call through `unsafe` raw-pointer/FFI boundaries into
//! game structures (`ManagerTemplate<AIHero>` and friends) that this sandbox
//! cannot fabricate a realistic live instance of. Task 20's manual in-game
//! checklist is where this code's correctness is actually verified — the port
//! stays as literal a transcription of the original as possible specifically
//! because of that.
//!
//! - [`apply_frame`] — the per-frame re-sync + one-shot saved-choice apply.
//! - [`handle_keydown`] — the raw-key half of `wndProc`.

mod apply_frame;
mod handle_keydown;

pub use apply_frame::apply_frame;
pub use handle_keydown::handle_keydown;
