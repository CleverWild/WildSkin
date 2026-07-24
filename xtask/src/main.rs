//! `cargo xtask build [--release]`: builds the workspace and reports where the
//! DLL and injector exe ended up.

mod cmd_build;

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
}

fn main() {
    let result = match Cli::parse().command {
        Commands::Build(args) => {
            cmd_build::run(&args).map_err(|e| format!("xtask build failed: {e}"))
        }
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

pub trait StreamingCommandExt {
    /// Runs a `cargo --message-format=json` build to completion, printing each
    /// `compiler-message`'s rendered text live. stderr is inherited (cargo's
    /// progress); stdout is piped (the JSON stream). Returns every raw line for
    /// the caller to search for artifact paths.
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
