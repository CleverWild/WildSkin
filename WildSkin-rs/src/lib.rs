// The `[lib] name` is "WildSkin" (PascalCase) rather than snake_case
// because the output filename must be exactly `WildSkin.dll` — the
// separate `WildSkin-injector` crate loads this exact filename.
#![allow(
    non_snake_case,
    reason = "the [lib] name is \"WildSkin\" (PascalCase) so the output filename matches WildSkin.dll exactly"
)]
#![cfg_attr(
    test,
    allow(
        clippy::undocumented_unsafe_blocks,
        clippy::multiple_unsafe_ops_per_block,
        reason = "many of tests intended to test unsafety (To win, take the lead)"
    )
)]

// Re-exported so the vendored `offset!` macro (see `offset.rs`) can refer to
// it as `$crate::paste::paste!`, matching how the crate it was vendored from
// re-exported its own `paste` dependency the same way.

#[macro_use]
pub(crate) mod offset;
mod config;
mod crypt;
mod entry;
mod fnv;
mod gui;
mod keybind;
mod logger;
mod memory;
mod overlay;
mod sdk;
mod skin_database;
mod skin_logic;
mod state;

use hudhook::windows::{
    Win32::{
        Foundation::{HINSTANCE, HMODULE},
        System::{
            LibraryLoader::{
                DisableThreadLibraryCalls, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_PIN, GetModuleFileNameW, GetModuleHandleExW,
            },
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
    core::PCWSTR,
};
use winsafe::CallNextHookEx;

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(
    hmodule: HINSTANCE,
    reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    if reason == DLL_PROCESS_ATTACH && is_target_process() {
        // The `SetWindowsHookEx` injector's own hook is what's keeping
        // this module mapped in the target process; once it calls
        // `UnhookWindowsHookEx`, Windows is free to unload us out from
        // under the background worker thread `entry::attach` spawns below.
        // Pinning permanently increments the loader's reference count so
        // this module survives for the rest of the process's lifetime.
        pin_module();
        // SAFETY: `hmodule` is the real module handle passed by the loader
        // for this DLL_PROCESS_ATTACH notification.
        let _ = unsafe { DisableThreadLibraryCalls(HMODULE(hmodule.0.cast())) };
        entry::attach(hmodule);
    }
    1
}

fn pin_module() {
    // SAFETY: `DllMain`'s own address is a valid code pointer within this
    // same module, used only as a lookup key for `GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS`.
    let _ = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            PCWSTR(DllMain as *const u16),
            &mut HMODULE::default(),
        )
    };
}

/// The `SetWindowsHookEx` injector loads this DLL into whichever process
/// owns the target thread it hooked. `LoadLibraryW` (called by the injector
/// itself, in its own process, to resolve `HookProc`'s address) also
/// triggers this same `DllMain`/`DLL_PROCESS_ATTACH` — so without this
/// guard, the skin-changer's startup sequence would run inside the injector's own
/// process too, hanging forever in `wait_for_game_client`.
fn is_target_process() -> bool {
    let mut buf = [0u16; 260];
    // SAFETY: `buf` is a valid, writable, correctly-sized wide-char buffer;
    // `None` requests the current process's own executable path.
    let len = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..len])
        .rsplit(['\\', '/'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(shared::GAME_PROCESS_NAME))
}

/// Exported hook procedure the `SetWindowsHookEx`-based injector resolves
/// by name. Installing this hook on a thread in the target process is what
/// makes Windows map this DLL there in the first place — `DllMain` above
/// does the real work once that happens, so this only needs to chain to
/// the next hook in the chain, as required of any `WH_GETMESSAGE` hook.
#[unsafe(no_mangle)]
extern "system" fn HookProc(code: i32, wparam: usize, lparam: isize) -> isize {
    CallNextHookEx(code, wparam, lparam)
}
