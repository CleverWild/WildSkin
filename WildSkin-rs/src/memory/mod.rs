//! Live-process signature scanning and the resolved offsets it produces.
//!
//! - [`scanner`] — the low-level AOB pattern matcher + PE `.text` locator.
//! - `signatures` — the AOB patterns for this game build.
//! - `offsets` — [`ResolvedOffsets`] and its typed accessors.
//! - `resolve` — the two-phase resolution (`wait_for_game_client` +
//!   `resolve_all`) that fills a `ResolvedOffsets`.

pub mod scanner;

mod offsets;
mod resolve;
mod signatures;

pub use offsets::ResolvedOffsets;
pub use resolve::{resolve_all, wait_for_game_client};
