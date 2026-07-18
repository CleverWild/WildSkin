//! `cargo xtask build [--release]` — builds the whole workspace and reports
//! where the skin-changer DLL and the injector exe ended up.
//!
//! `cargo xtask setup-ollvm` — downloads and links the obfuscation toolchain
//! used by `build --obfuscate`; safe to run in CI or on a fresh machine.

mod cmd_build;
mod cmd_setup_ollvm;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build(cmd_build::BuildArgs),
    /// Downloads and `rustup toolchain link`s the `ollvm` toolchain used by `build --obfuscate`.
    SetupOllvm,
}

fn main() {
    let result = match Cli::parse().command {
        Commands::Build(args) => {
            cmd_build::run(&args).map_err(|e| format!("xtask build failed: {e}"))
        }
        Commands::SetupOllvm => {
            cmd_setup_ollvm::run().map_err(|e| format!("xtask setup-ollvm failed: {e}"))
        }
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
