use hudhook::{hooks::dx11::ImguiDx11Hooks, windows::Win32::Foundation::HINSTANCE};
use winsafe::prelude::*;

/// Mirrors the original's `HideThread(GetCurrentThread())`: hides the real
/// worker thread from a debugger via `NtSetInformationThread`
/// (`ThreadHideFromDebugger`). Unlike the original's `DllMain`, this is NOT
/// called with a module handle — see the module deviation note in the task
/// plan for why that half of the original call pair is dropped.
fn hide_thread(thread: &winsafe::HTHREAD) -> bool {
    type NtSetInformationThreadFn =
        unsafe extern "system" fn(*mut std::ffi::c_void, u32, *const std::ffi::c_void, u32) -> i32;

    // ntdll is always loaded in every Windows process.
    let Ok(ntdll) = winsafe::HINSTANCE::GetModuleHandle(Some("ntdll.dll")) else {
        return false;
    };
    let Ok(proc) = ntdll.GetProcAddress("NtSetInformationThread") else {
        return false;
    };
    // SAFETY: `proc` was just resolved from `ntdll.dll`'s real export table
    // and matches the well-known `NtSetInformationThread` signature.
    let func: NtSetInformationThreadFn = unsafe { std::mem::transmute(proc) };
    // ThreadHideFromDebugger = 0x11
    // SAFETY: `thread` is a valid, currently-open thread handle; `func` is
    // the real `NtSetInformationThread` resolved above.
    (unsafe { func(thread.ptr(), 0x11, std::ptr::null(), 0) }) == 0
}

pub fn attach(hmodule: HINSTANCE) {
    // HINSTANCE wraps a raw pointer and is not Send, so it can't be captured
    // directly by the spawned closure. Ferry it across as a usize and
    // reconstruct it on the worker thread, same as hudhook's own `hudhook!`
    // macro does internally.
    let hmodule_raw = hmodule.0 as usize;
    std::thread::spawn(move || {
        let hmodule = HINSTANCE(hmodule_raw as _);

        let current_thread = winsafe::HTHREAD::GetCurrentThread();
        let _ = hide_thread(&current_thread);

        // SAFETY: this worker thread runs inside the target game process,
        // as required by `wait_for_game_client`.
        let (base, game_client) = unsafe { crate::memory::wait_for_game_client() };

        // Original's `DllAttach` sleeps briefly between the two search
        // phases to let the game settle after reaching `Running`.
        std::thread::sleep(std::time::Duration::from_millis(500));

        // SAFETY: `base`/`game_client` were just returned by
        // `wait_for_game_client`, satisfying `resolve_all`'s contract.
        let Some(offsets) = (unsafe { crate::memory::resolve_all(base, game_client) }) else {
            let _ = winsafe::HWND::NULL.MessageBox(
                "Failed to resolve one or more signatures.",
                shared::APP_NAME,
                winsafe::co::MB::OK,
            );
            hudhook::eject();
            return;
        };
        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut database = crate::app::skin_database::SkinDatabase::empty();
        // SAFETY: `offsets` was just resolved against this same live process
        // by `resolve_all` above.
        let champion_manager = unsafe { offsets.champion_manager() };
        // SAFETY: `champion_manager` was just resolved above, against this
        // same live process.
        unsafe {
            database.load(champion_manager, &offsets);
        }

        let mut config = crate::app::config::Config::default();
        // SAFETY: the player, if present, was just resolved above against
        // this same live process.
        let player_model = unsafe { offsets.player_ref() }.map(|p_ref| {
            // SAFETY: `character_data_stack`'s offset was resolved against
            // this same live process.
            let stack =
                unsafe { p_ref.character_data_stack_ref(offsets.fields.character_data_stack) };
            // SAFETY: `stack.base_skin.model` is a live, initialized field.
            unsafe { stack.base_skin.model.as_str() }.to_owned()
        });
        config.load(&crate::app::config::config_dir(), player_model.as_deref());

        crate::state::init(offsets, config, database);

        if let Err(e) = hudhook::Hudhook::builder()
            .with::<ImguiDx11Hooks>(crate::gui::overlay::Overlay)
            .with_hmodule(hmodule)
            .build()
            .apply()
        {
            hudhook::tracing::error!("Couldn't apply hooks: {e:?}");
            hudhook::eject();
        }
    });
}
