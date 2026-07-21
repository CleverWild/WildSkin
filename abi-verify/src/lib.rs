pub mod arg_count;
pub mod full_signature;
pub mod pe;
pub mod resolve;

/// Path to the game exe `#[verify_abi]` checks against; unset means skip.
pub const EXE_PATH_ENV_VAR: &str = "WILDSKIN_LEAGUE_EXE_PATH";

/// Set to `true`/`1` (CI does) to make `#[verify_abi]` a pure passthrough.
pub const DISABLED_ENV_VAR: &str = "WILDSKIN_SKIP_ABI_VERIFY";

/// Whether [`DISABLED_ENV_VAR`] is set to a truthy value.
pub fn verification_disabled() -> bool {
    std::env::var(DISABLED_ENV_VAR)
        .is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}
