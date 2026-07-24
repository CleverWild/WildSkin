//! Constants that must agree between the injector (`WildSkin-injector`) and the
//! DLL it loads (`WildSkin-rs`), kept in one place so they can't drift apart.

/// The DLL's output filename (fixed by `WildSkin-rs`'s `[lib] name`); the
/// injector looks for this exact file and loaded-module name.
///
/// A fn, not a `const &str`: `obfstr` XOR-obfuscates the literal at compile
/// time so it isn't plaintext in `.rdata` for a static string scan.
pub fn dll_file_name() -> String {
    obfstr::obfstr!("WildSkin.dll").to_owned()
}

/// Name of the `WH_GETMESSAGE` hook procedure the DLL exports.
///
/// The injector resolves this to install the hook that gets the DLL mapped
/// into the game process. Obfuscated for the same reason as [`dll_file_name`].
pub fn hook_proc_name() -> String {
    obfstr::obfstr!("HookProc").to_owned()
}

/// The game process name; the DLL checks it was loaded into the right target,
/// the injector uses it to find candidate processes to hook.
pub const GAME_PROCESS_NAME: &str = "League of Legends.exe";

/// The League client process; the injector reports client-found status in GUI.
pub const CLIENT_PROCESS_NAME: &str = "LeagueClient.exe";

/// The product's display name.
pub const APP_NAME: &str = "WildSkin";

/// Default League install location, probed only when the var is unset.
pub const STANDARD_LOL_PATH: &str = r"C:\Riot Games\League of Legends\Game\League of Legends.exe";
