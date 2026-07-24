//! Live-process signature scanning and the [`ResolvedOffsets`] it produces.
//! `scanner` matches AOB patterns in `.text`; `resolve` fills a
//! `ResolvedOffsets` in two phases (`wait_for_game_client` + `resolve_all`).

pub mod scanner;

mod offsets;
mod resolve;
mod signatures;

pub use offsets::ResolvedOffsets;
pub use resolve::{resolve_all, wait_for_game_client};
