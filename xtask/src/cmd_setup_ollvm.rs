use std::io::{Cursor, Read as _};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pinned release of <https://github.com/ParkSnoopy/rust_llvm-arkari_ollvm>: a
/// full `rustc` (1.96.0-dev) on custom LLVM 22.1 with Arkari `irobf` passes
/// baked in, so no `-Zllvm-plugins` DLL is needed. Pinned, not "latest": the
/// flags are version-sensitive (an LLVM patch can change which `irobf-*` are
/// stable; see `cmd_build.rs`'s `OBFUSCATION_LLVM_ARGS`).
const RELEASE_TAG: &str = "R1940-L2210";
const RELEASE_ASSET: &str = "x86_64-pc-windows-msvc.zip";
const TOOLCHAIN_NAME: &str = "ollvm";

/// Downloads and links the toolchain if absent, so `cargo xtask build
/// --obfuscate` is reproducible on a fresh machine or CI without a by-hand
/// `rustup toolchain link`.
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

/// Under the workspace `target` dir so it shares the same gitignore and CI
/// cache scope as other artifacts, not a new machine-global location.
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
    // Idempotent: `rustup toolchain link` overwrites an existing link, so no
    // "already linked" check is needed (unlike download/extract above).
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
