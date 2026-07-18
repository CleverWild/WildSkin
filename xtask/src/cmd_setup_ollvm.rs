use std::io::{Cursor, Read as _};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pinned release of <https://github.com/ParkSnoopy/rust_llvm-arkari_ollvm> —
/// a full `rustc` build (Rust 1.96.0-dev) linked against a custom LLVM 22.1
/// with the Arkari (`KomiMoe/Arkari`) `irobf` obfuscation passes baked in
/// natively, so no `-Zllvm-plugins` plugin DLL is needed. Pinned rather than
/// "latest": the obfuscation flags are version-sensitive (a plugin API
/// mismatch or a different LLVM patch can silently change which `irobf-*`
/// flags are stable — `--irobf-fla` already crashes `rustc_driver` on an
/// `APInt` assertion against this exact build when combined with
/// `windows`-crate-heavy code, see `cmd_build.rs`'s `OBFUSCATION_LLVM_ARGS`
/// doc comment).
const RELEASE_TAG: &str = "R1940-L2210";
const RELEASE_ASSET: &str = "x86_64-pc-windows-msvc.zip";
const TOOLCHAIN_NAME: &str = "ollvm";

/// Downloads and links the toolchain if it isn't already set up — makes
/// `cargo xtask build --obfuscate` reproducible on a fresh machine or CI
/// runner instead of depending on a by-hand `rustup toolchain link`.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let install_dir = toolchain_dir()?;
    let stage1 = install_dir.join("stage1");
    if stage1.join("bin").join("rustc.exe").is_file() {
        println!("ollvm toolchain already extracted at {}", stage1.display());
    } else {
        download_and_extract(&install_dir)?;
    }
    link_toolchain(&stage1)?;
    Ok(())
}

/// Workspace-`target`-relative so it's covered by the same gitignore and CI
/// cache scope as every other build artifact, rather than inventing a new
/// machine-global location.
fn toolchain_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()?;
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let target_directory = metadata["target_directory"]
        .as_str()
        .ok_or("cargo metadata did not report a target_directory")?;
    Ok(PathBuf::from(target_directory).join("ollvm-toolchain"))
}

fn download_and_extract(install_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://github.com/ParkSnoopy/rust_llvm-arkari_ollvm/releases/download/{RELEASE_TAG}/{RELEASE_ASSET}"
    );
    println!("Downloading {url} ...");
    let mut response = ureq::get(&url).call()?;
    let mut zip_bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut zip_bytes)?;

    println!("Extracting to {} ...", install_dir.display());
    std::fs::create_dir_all(install_dir)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))?;
    archive.extract(install_dir)?;
    Ok(())
}

fn link_toolchain(stage1: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Idempotent: `rustup toolchain link` overwrites an existing link of the
    // same name rather than erroring, so no separate "already linked" check
    // is needed here (unlike the download/extract step above).
    let status = Command::new("rustup")
        .args(["toolchain", "link", TOOLCHAIN_NAME])
        .arg(stage1)
        .status()?;
    if !status.success() {
        return Err("rustup toolchain link failed".into());
    }
    println!(
        "Linked toolchain '{TOOLCHAIN_NAME}' -> {}",
        stage1.display()
    );
    Ok(())
}
