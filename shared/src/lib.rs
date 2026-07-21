//! Constants that must agree between the injector (`WildSkin-injector`) and
//! the skin-changer DLL it loads (`WildSkin-rs`) — kept in one place so the two
//! crates can't silently drift apart on a name only one side updates.

/// The skin-changer DLL's output filename. Fixed by `WildSkin-rs`'s `[lib] name`
/// in its `Cargo.toml`; the injector looks for this exact file on disk and
/// by this exact loaded-module name.
///
/// A function rather than a `const &str`: the literal is XOR-obfuscated at
/// compile time (`obfstr`) and decrypted at each call, so it doesn't sit as
/// plaintext in the injector's `.rdata` for a static string scan to find.
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

/// The League of Legends game process; the DLL uses this to recognize
/// whether it was loaded into the intended target, and the injector uses it
/// to find candidate processes to hook.
pub const GAME_PROCESS_NAME: &str = "League of Legends.exe";

/// The League client process; only the injector needs this today, to report
/// client-found status in its GUI.
pub const CLIENT_PROCESS_NAME: &str = "LeagueClient.exe";

/// The product's display name.
pub const APP_NAME: &str = "WildSkin";

/// Default League install location, probed only when the var is unset.
pub const STANDARD_LOL_PATH: &str = r"C:\Riot Games\League of Legends\Game\League of Legends.exe";
