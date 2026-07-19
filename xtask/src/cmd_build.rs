use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;

use crate::StreamingCommandExt as _;

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

/// Builds the DLL and prints the DLL/exe paths.
///
/// `wildskin-injector` is a private, git-ignored *separate* workspace (its own
/// `Cargo.toml`/`Cargo.lock`, absent from a plain clone of this public repo),
/// so it's built by manifest path when present — not via the root `--workspace`.
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

    /// Sets output to temp dir instead of `target/`.
    #[arg(long)]
    temp: bool,

    /// Opens the output dir. Combined with `--temp`, the dir is deleted once you close that window.
    #[arg(short, long)]
    open: bool,
}

pub fn run(args: &BuildArgs) -> Result<(), Box<dyn std::error::Error>> {
    let has_injector = std::path::Path::new("WildSkin-injector").is_dir();

    if args.obfuscate {
        crate::cmd_setup_ollvm::run()?;
    }

    // DLL: root workspace. Injector: its own private, git-ignored workspace,
    // built by manifest path (not as a member); `find_artifact` reads the
    // absolute paths out of cargo's JSON, so the target location doesn't matter.
    let dll_json = run_cargo_build(args, &["-p", "wildskin", "--lib"])?;
    let dll_path = find_artifact(dll_json.as_bytes(), "cdylib", "WildSkin")
        .ok_or("could not find WildSkin.dll in cargo's build output")?;
    println!("Built:  {}", dll_path.display());

    let exe_path = if has_injector {
        let exe_json =
            run_cargo_build(args, &["--manifest-path", "WildSkin-injector/Cargo.toml"])?;
        let exe_path = find_artifact(exe_json.as_bytes(), "bin", "WildSkin_Injector")
            .ok_or("could not find WildSkin_Injector.exe in cargo's build output")?;
        println!("Built:  {}", exe_path.display());
        Some(exe_path)
    } else {
        println!("Skipped: WildSkin-injector (private component not checked out here)");
        None
    };

    let output_dir = if args.temp {
        std::env::temp_dir().join("wildskin-build")
    } else {
        dll_path.parent().unwrap().to_path_buf()
    };

    if args.temp {
        std::fs::create_dir_all(&output_dir)?;
        std::fs::copy(&dll_path, output_dir.join(dll_path.file_name().unwrap()))?;
        if let Some(exe_path) = &exe_path {
            std::fs::copy(exe_path, output_dir.join(exe_path.file_name().unwrap()))?;
        }
        println!("Copied to: {}", output_dir.display());
    }

    if args.temp && args.open {
        // Browse-then-discard: delete the temp dir once the user closes its window.
        if let Err(e) = spawn_temp_cleanup_watcher(&output_dir) {
            eprintln!("warning: could not start cleanup watcher: {e}");
        } else {
            println!(
                "Opened {}; it will be deleted when you close the Explorer window.",
                output_dir.display()
            );
        }
    } else if args.open {
        // Fire-and-forget: explorer.exe often exits nonzero even on success.
        let _ = Command::new("explorer").arg(&output_dir).spawn();
    }

    Ok(())
}

/// Runs one `cargo build` (root DLL or the injector's separate workspace) with
/// the profile/obfuscation flags from `args`, returning its
/// `--message-format=json` output joined by newlines.
fn run_cargo_build(
    args: &BuildArgs,
    target_args: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cargo_args: Vec<&str> = vec![];
    if args.obfuscate {
        cargo_args.push("+ollvm");
    }
    cargo_args.push("build");
    cargo_args.extend_from_slice(target_args);
    if args.release || args.obfuscate {
        cargo_args.push("--release");
    }

    println!("=== cargo {} ===", cargo_args.join(" "));
    let mut command = Command::new("cargo");
    command.args(&cargo_args);
    // `cargo xtask` is `cargo run -p xtask`, which sets `CARGO_MANIFEST_DIR`
    // and the `CARGO_PKG_*`/`CARGO_BIN_NAME`/`CARGO_CRATE_NAME` family to
    // describe *xtask's own* package for this process. Left in place, this
    // nested `cargo build` inherits them, and a stray `CARGO_MANIFEST_DIR`
    // (xtask's dir instead of absent) flips `ring`'s build-script fingerprint
    // on every single invocation, forcing it — and everything depending on
    // it (`rustls`, `ureq`, ...) — to recompile from scratch every time.
    // (Deliberately narrow: unlike a blanket `CARGO_`-prefix strip, this
    // leaves `CARGO_HOME`/`CARGO_TARGET_DIR`/etc. untouched.)
    for key in [
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_PATH",
        "CARGO_PKG_NAME",
        "CARGO_PKG_VERSION",
        "CARGO_PKG_VERSION_MAJOR",
        "CARGO_PKG_VERSION_MINOR",
        "CARGO_PKG_VERSION_PATCH",
        "CARGO_PKG_VERSION_PRE",
        "CARGO_PKG_AUTHORS",
        "CARGO_PKG_DESCRIPTION",
        "CARGO_PKG_HOMEPAGE",
        "CARGO_PKG_REPOSITORY",
        "CARGO_PKG_LICENSE",
        "CARGO_PKG_LICENSE_FILE",
        "CARGO_PKG_RUST_VERSION",
        "CARGO_PKG_README",
        "CARGO_CRATE_NAME",
        "CARGO_BIN_NAME",
        "CARGO_PRIMARY_PACKAGE",
    ] {
        command.env_remove(key);
    }
    if args.obfuscate {
        let rustflags = OBFUSCATION_LLVM_ARGS
            .iter()
            .map(|flag| format!("-Cllvm-args={flag}"))
            .collect::<Vec<_>>()
            .join(" ");
        command.env("RUSTFLAGS", rustflags);
    }

    Ok(command.run_rendering_cargo_json()?.join("\n"))
}

/// The `--temp --open` watcher script. Opens `-Dir` in Explorer, waits for that
/// window to close, then deletes the folder. It pins the folder by an OS handle
/// (`GetFinalPathNameByHandle`), not its name, so a rename in Explorer is still
/// cleaned up — Explorer's own reported path goes stale after a rename, the
/// handle doesn't. Run via `-File` so no cross-language quoting is needed.
const CLEANUP_WATCHER_PS1: &str = include_str!("../cleanup-watcher.ps1");

/// Writes [`CLEANUP_WATCHER_PS1`] to a fixed temp path and launches it detached
/// and hidden, so the build command returns immediately.
fn spawn_temp_cleanup_watcher(dir: &Path) -> std::io::Result<()> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script_path = std::env::temp_dir().join("wildskin-cleanup-watch.ps1");
    std::fs::write(&script_path, CLEANUP_WATCHER_PS1)?;
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script_path)
        .arg("-Dir")
        .arg(dir)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
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
