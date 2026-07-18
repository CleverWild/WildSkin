# WildSkin

> [!IMPORTANT]
> Please do not use Issues to post advertisements, promotions, or repeated recommendations of similar paid products. Whether those products work or not is unrelated to this project, but abusive language, spam, or repeatedly posting through alternate accounts will not be accepted.

## Disclaimer

**This project is for learning and technical exchange purposes only. Commercial use or any illegal activity is strictly prohibited. Any direct or indirect consequences arising from the use of this project shall be borne solely by the user, and the author assumes no responsibility.**

**By using this project, you fully understand and accept the above terms.**

## Building

Requirements: a recent stable [Rust](https://rustup.rs/) toolchain (edition
2024, latest) targeting `x86_64-pc-windows-msvc`.

```bash
git clone https://github.com/CleverWild/WildSkin.git
cd WildSkin
cargo xtask build --release
```

This builds `WildSkin.dll` and prints where it ended up. The injector
(`WildSkin_Injector.exe`) is a separate, closed-source component not built
from this repository; grab it from
[Releases](https://github.com/CleverWild/WildSkin/releases/latest).

If you don't trust this project's closed-source injector, the injector from
[hydy100/R3nzSkin](https://github.com/hydy100/R3nzSkin)'s releases can be
used instead, those two are interchangeable.

### `cargo xtask` reference

All builds go through the `xtask` crate rather than plain `cargo build`, so
the DLL/exe output paths get resolved and printed for you instead of having
to dig through `target/`.

**`cargo xtask build [flags]`**

| Flag | Effect |
| --- | --- |
| `-r`, `--release` | Release profile (optimized, LTO, `panic = "abort"`) instead of the faster debug profile. |
| `--obfuscate` | Builds with the `ollvm` toolchain and Arkari `irobf` obfuscation passes; implies `--release`. |
| `--temp` | Copies the built DLL/exe to a temp dir instead of leaving them under `target/`. |
| `-o`, `--open` | Opens the output dir in Explorer after building. Combined with `--temp`, the temp dir is deleted once you close that Explorer window. |

`WildSkin_Injector.exe` is built alongside the DLL only when
`WildSkin-injector/` is checked out; a plain public-repo clone builds just
the DLL.

**`cargo xtask setup-ollvm`**

Downloads and `rustup toolchain link`s the pinned `ollvm` toolchain that
`build --obfuscate` needs. Idempotent (skips the download if already
extracted). `cargo xtask build --obfuscate` runs it automatically.

## Usage

> [!NOTE]
> Neither `WildSkin.dll` nor `WildSkin_Injector.exe` require disabling Windows Defender
> or most other antivirus software. Both are tested clean on VirusTotal
> and local with Defender's all flags left on.

1. Build from source (above) or grab a build from
   [Releases](https://github.com/CleverWild/WildSkin/releases/latest), once
   one is published.
2. Run `WildSkin_Injector.exe` and click **Start** while League of Legends
   is running (or launch the injector first — it waits for and auto-detects
   the game process).
3. The menu opens automatically; press **Insert** (default keybind,
   rebindable) to toggle it. Enable **Quick Skin Change** in the Extras tab
   to cycle skins with **Page Up**/**Page Down** (also rebindable).

## About the project

WildSkin is a full Rust rewrite of the original C++ `R3nzSkin`, whose
[upstream repository](https://github.com/R3nzTheCodeGOD/R3nzSkin) is now
archived; this project is maintained as a fork independent of the upstream
C++ codebases. The skin-changer DLL (`WildSkin-rs`, this repository) is open
source; there's no paid version. The injector is a separate, closed-source
component.

## Credits

This project is a Rust port of the original C++ repository [R3nzTheCodeGOD/R3nzSkin](https://github.com/R3nzTheCodeGOD/R3nzSkin), which is distributed under the MIT license (see the [`LICENSE-ORIGINAL`](LICENSE-ORIGINAL) file).

Thanks to [hydy100/R3nzSkin](https://github.com/hydy100/R3nzSkin) for the inspiration behind this project's injector.

The skin-changer DLL (`WildSkin-rs`) is licensed under the terms of the **GNU
General Public License v3.0** (see the [`LICENSE-GPL`](LICENSE-GPL) file). The
supporting crates (`shared`, `xtask`, `abi-verify`, `abi-verify-macro`) are
dual-licensed under **MIT** or **Apache-2.0**, at your option (see
[`LICENSE-MIT`](LICENSE-MIT) / [`LICENSE-APACHE`](LICENSE-APACHE)).
