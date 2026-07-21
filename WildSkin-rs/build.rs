//! Points `#[verify_abi]` at a standard League install so a dev machine needs
//! no configuration. Lives here, not in the macro, because `cargo:rustc-env`
//! reaches only this crate's compilation — leaving `abi-verify-macro`'s own
//! tests, which rely on the var being unset, unaffected.

use abi_verify::EXE_PATH_ENV_VAR;
use shared::STANDARD_LOL_PATH;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed={EXE_PATH_ENV_VAR}");

    // An explicit override always wins; a bad path there is the macro's to report.
    if std::env::var_os(EXE_PATH_ENV_VAR).is_some() {
        return;
    }
    // Left unset when absent — that's what makes the macro warn.
    if Path::new(STANDARD_LOL_PATH).exists() {
        println!("cargo:rustc-env={EXE_PATH_ENV_VAR}={STANDARD_LOL_PATH}");
        println!("cargo:rerun-if-changed={STANDARD_LOL_PATH}");
    }
}
