//! `cargo xtask build [--release]` — builds the whole workspace and reports
//! where the skin-changer DLL and the injector exe ended up.
//!
//! `cargo xtask setup-ollvm` — downloads and links the obfuscation toolchain
//! used by `build --obfuscate`; safe to run in CI or on a fresh machine.

mod cmd_build;
mod cmd_setup_ollvm;

use std::{
    io::{
        BufReader,
        prelude::{BufRead as _, Write as _},
    },
    process::{Command, Stdio},
};

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

pub trait StreamingCommandExt {
    /// Runs a `cargo --message-format=json` invocation to completion,
    /// printing each `compiler-message`'s rendered text live as it arrives.
    /// stderr is inherited so cargo's own "Compiling ..." progress reaches
    /// the console in real time; stdout is piped since that's where
    /// `--message-format=json` writes its one-JSON-object-per-line stream.
    /// Returns every raw line for the caller to search for artifact paths.
    fn run_rendering_cargo_json(&mut self) -> std::io::Result<Vec<String>>;
}

impl StreamingCommandExt for Command {
    fn run_rendering_cargo_json(&mut self) -> std::io::Result<Vec<String>> {
        self.arg("--message-format=json,json-diagnostic-rendered-ansi");
        let mut child = self
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut json_lines = Vec::new();
        for line in stdout.lines() {
            let line = line?;
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line)
                && msg.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-message")
                && let Some(rendered) = msg
                    .get("message")
                    .and_then(|m| m.get("rendered"))
                    .and_then(serde_json::Value::as_str)
            {
                print!("{rendered}");
                std::io::stdout().flush()?;
            }
            json_lines.push(line);
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(std::io::Error::other("cargo build failed"));
        }
        Ok(json_lines)
    }
}
