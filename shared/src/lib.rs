//! Constants that must agree between the injector (`WildSkin-injector`) and
//! the skin-changer DLL it loads (`WildSkin-rs`) — kept in one place so the two
//! crates can't silently drift apart on a name only one side updates.

/// The skin-changer DLL's output filename. Fixed by `WildSkin-rs`'s `[lib] name`
/// in its `Cargo.toml`; the injector looks for this exact file on disk and
/// by this exact loaded-module name.
pub const DLL_FILE_NAME: &str = "WildSkin.dll";

/// Name of the `WH_GETMESSAGE` hook procedure the DLL exports and the
/// injector resolves via `GetProcAddress` to install the hook that gets it
/// mapped into the game process.
pub const HOOK_PROC_NAME: &str = "HookProc";

/// The League of Legends game process; the DLL uses this to recognize
/// whether it was loaded into the intended target, and the injector uses it
/// to find candidate processes to hook.
pub const GAME_PROCESS_NAME: &str = "League of Legends.exe";

/// The League client process; only the injector needs this today, to report
/// client-found status in its GUI.
pub const CLIENT_PROCESS_NAME: &str = "LeagueClient.exe";

/// The product's display name.
pub const APP_NAME: &str = "WildSkin";
