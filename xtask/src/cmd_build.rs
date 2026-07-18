use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::Args;

/// The `irobf` flag set applied to both crates by `--obfuscate`.
///
/// `--irobf-fla` (control-flow flattening) is deliberately excluded: it
/// crashes `rustc_driver.dll` (`APInt.h:1566`,
/// `getSignificantBits() <= 64 && "Too many bits for int64_t"`) while
/// compiling `wildskin`, which pulls in `windows`-crate code through
/// `hudhook` — confirmed by bisecting against the same build, not just
/// going on the upstream toolchain's own README warning about
/// `windows-rs`/`rand`/`clap`. Re-adding it needs re-verifying against
/// whatever code triggered the crash first.
const OBFUSCATION_LLVM_ARGS: &[&str] = &[
    "--irobf",
    "--irobf-indbr",
    "--irobf-icall",
    "--irobf-indgv",
    "--irobf-cse",
];

/// Builds the workspace and prints the DLL/exe paths.
///
/// `wildskin-injector` is only a workspace member on machines that have it
/// checked out and added to the local (git-ignored-by-`skip-worktree`)
/// `Cargo.toml`; a plain clone of this public repo doesn't have the
/// directory at all, so it's built conditionally, not via `--workspace`.
#[derive(Args)]
pub struct BuildArgs {
    /// Build in release mode (optimized, `panic = "abort"`, LTO) instead of the faster debug profile.
    #[arg(short, long)]
    release: bool,

    /// Build with the `ollvm` toolchain (`cargo xtask setup-ollvm` links it)
    /// and the Arkari `irobf` obfuscation passes — implies `--release` (the
    /// flags are meant for hardened distribution builds, not day-to-day
    /// development). See `OBFUSCATION_LLVM_ARGS` for the exact pass set.
    #[arg(long)]
    obfuscate: bool,

    /// Sets output to temp dir and opens it.
    #[arg(long)]
    temp: bool,
}

pub fn run(args: &BuildArgs) -> Result<(), Box<dyn std::error::Error>> {
    let has_injector = std::path::Path::new("WildSkin-injector").is_dir();

    let mut cargo_args = vec![];
    if args.obfuscate {
        cargo_args.push("+ollvm");
    }
    cargo_args.extend(["build", "-p", "wildskin", "--lib"]);
    if has_injector {
        cargo_args.extend(["-p", "wildskin-injector", "--bin", "WildSkin_Injector"]);
    }
    cargo_args.push("--message-format=json");
    if args.release || args.obfuscate {
        cargo_args.push("--release");
    }

    println!("=== cargo {} (workspace) ===", cargo_args.join(" "));
    let mut command = Command::new("cargo");
    command.args(&cargo_args);
    if args.obfuscate {
        let rustflags = OBFUSCATION_LLVM_ARGS
            .iter()
            .map(|flag| format!("-Cllvm-args={flag}"))
            .collect::<Vec<_>>()
            .join(" ");
        command.env("RUSTFLAGS", rustflags);
    }
    // stderr (cargo's "Compiling ..." progress) goes straight to the
    // console for real-time output; stdout (the JSON message stream) is
    // piped so diagnostics can be rendered live and artifacts collected.
    command.stdout(Stdio::piped()).stderr(Stdio::inherit());
    let mut child = command.spawn()?;
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
        return Err("cargo build failed".into());
    }
    let json_output = json_lines.join("\n");

    let dll_path = find_artifact(json_output.as_bytes(), "cdylib", "WildSkin")
        .ok_or("could not find WildSkin.dll in cargo's build output")?;
    println!("Built:  {}", dll_path.display());
    let exe_path = if has_injector {
        let exe_path = find_artifact(json_output.as_bytes(), "bin", "WildSkin_Injector")
            .ok_or("could not find WildSkin_Injector.exe in cargo's build output")?;
        println!("Built:  {}", exe_path.display());
        Some(exe_path)
    } else {
        println!("Skipped: WildSkin-injector (private component not checked out here)");
        None
    };
    println!("Output: {}", dll_path.parent().unwrap().display());

    if args.temp {
        let temp_dir = std::env::temp_dir().join("wildskin-build");
        std::fs::create_dir_all(&temp_dir)?;
        std::fs::copy(&dll_path, temp_dir.join(dll_path.file_name().unwrap()))?;
        if let Some(exe_path) = &exe_path {
            std::fs::copy(exe_path, temp_dir.join(exe_path.file_name().unwrap()))?;
        }
        println!("Copied to: {}", temp_dir.display());
        // Fire-and-forget: explorer.exe often exits nonzero even on success.
        let _ = Command::new("explorer").arg(&temp_dir).spawn();
    }
    Ok(())
}

/// Cargo's `--message-format=json` output is one JSON object per line; the
/// `compiler-artifact` message for a build target lists its real output
/// path(s) directly. Filtering by `target.kind` alone isn't enough: `xtask`
/// is also a `bin` target sharing this same workspace build, so the target
/// name must match too, or a search for "the bin artifact" can just as
/// easily grab xtask's own executable instead of the injector's.
fn find_artifact(json_output: &[u8], kind: &str, name: &str) -> Option<PathBuf> {
    for line in String::from_utf8_lossy(json_output).lines() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if msg.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-artifact") {
            continue;
        }
        let target = msg.get("target");
        let matches_kind = target
            .and_then(|t| t.get("kind"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|kinds| kinds.iter().any(|k| k.as_str() == Some(kind)));
        let matches_name = target
            .and_then(|t| t.get("name"))
            .and_then(serde_json::Value::as_str)
            == Some(name);
        if !matches_kind || !matches_name {
            continue;
        }
        let Some(filenames) = msg.get("filenames").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for filename in filenames {
            if let Some(path) = filename.as_str() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}
