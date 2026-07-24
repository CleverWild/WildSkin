// [lib] name is "WildSkin" (PascalCase) not snake_case so the output is
// exactly WildSkin.dll, the filename WildSkin-injector loads.
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

mod app;
mod entry;
mod gui;
mod memory;
mod sdk;
mod skin_logic;
mod state;
mod util;

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
        // The injector unhooks right after injecting, freeing Windows to
        // unload us from under the worker thread entry::attach spawns.
        // Pin the module so it survives for the process's lifetime.
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

/// The injector's own `LoadLibraryW` (resolving `HookProc`) also fires this
/// `DLL_PROCESS_ATTACH`; without this guard, startup would run inside the
/// injector too and hang forever in `wait_for_game_client`.
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

/// Exported hook proc the injector resolves by name; installing it is what
/// maps this DLL into the target. `DllMain` does the real work, so this
/// just chains to the next hook as a `WH_GETMESSAGE` hook must.
#[unsafe(no_mangle)]
extern "system" fn HookProc(code: i32, wparam: usize, lparam: isize) -> isize {
    CallNextHookEx(code, wparam, lparam)
}
