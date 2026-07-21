//! Points `#[verify_abi]` at a standard League install so a dev machine needs
//! no configuration. Lives here, not in the macro, because `cargo:rustc-env`
//! reaches only this crate's compilation — leaving `abi-verify-macro`'s own
//! tests, which rely on the var being unset, unaffected.

use abi_verify::EXE_PATH_ENV_VAR;
use shared::STANDARD_LOL_PATH;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed={EXE_PATH_ENV_VAR}");

    // An explicit override always wins; a bad path there is the macro's to report.
    // Left unset when no install is found — that's what makes the macro warn.
    let exe = match std::env::var(EXE_PATH_ENV_VAR) {
        Ok(path) => PathBuf::from(path),
        Err(_) if Path::new(STANDARD_LOL_PATH).exists() => {
            println!("cargo:rustc-env={EXE_PATH_ENV_VAR}={STANDARD_LOL_PATH}");
            PathBuf::from(STANDARD_LOL_PATH)
        }
        Err(_) => return,
    };
    // Re-verify after a game patch: an explicitly-set exe used to skip this and
    // stay stale, since the early return happened before the trigger was emitted.
    println!("cargo:rerun-if-changed={}", exe.display());
}
