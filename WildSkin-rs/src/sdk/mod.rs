// `offset!` lives here; `#[macro_export]` publishes it as `crate::offset!`
// regardless of file location.
mod offset;

pub mod ai_base_common;
pub mod champion;
pub mod character_data;
pub mod game_object;
pub mod game_state;
pub mod msvc_string;
pub mod primitives;
