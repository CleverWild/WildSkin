use crate::app::config::Config;
use crate::app::logger::Logger;
use crate::app::skin_database::SkinDatabase;
use crate::memory::ResolvedOffsets;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

pub struct AppState {
    pub offsets: ResolvedOffsets,
    pub config: Mutex<Config>,
    pub logger: Mutex<Logger>,
    pub database: SkinDatabase,
    pub menu_open: AtomicBool,
}

impl AppState {
    pub fn is_menu_open(&self) -> bool {
        self.menu_open.load(Ordering::Relaxed)
    }

    pub fn toggle_menu_open(&self) -> bool {
        // Atomic: the hotkey fires from the render-thread WndProc hook while
        // other frames may read the flag concurrently.
        let mut current = self.menu_open.load(Ordering::Relaxed);
        loop {
            let new = !current;
            match self.menu_open.compare_exchange_weak(
                current,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return new,
                Err(actual) => current = actual,
            }
        }
    }
}

static APP_STATE: OnceLock<AppState> = OnceLock::new();

pub fn init(offsets: ResolvedOffsets, config: Config, database: SkinDatabase) {
    let menu_open = AtomicBool::new(config.is_open);
    APP_STATE
        .set(AppState {
            offsets,
            config: Mutex::new(config),
            logger: Mutex::new(Logger::new()),
            database,
            menu_open,
        })
        .ok()
        .expect("AppState::init called more than once");
}

pub fn get() -> &'static AppState {
    APP_STATE.get().expect("AppState accessed before init()")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_offsets() -> ResolvedOffsets {
        // ResolvedOffsets has no public constructor; tests use the cfg(test)
        // dummy, which never dereferences globals or calls its fns.
        ResolvedOffsets::dummy_for_test()
    }

    fn dummy_app_state() -> AppState {
        AppState {
            offsets: dummy_offsets(),
            config: Mutex::new(Config::default()),
            logger: Mutex::new(Logger::new()),
            database: SkinDatabase::empty(),
            menu_open: AtomicBool::new(false),
        }
    }

    #[test]
    fn toggle_menu_open_flips_and_returns_the_new_state() {
        // Local instance, not the process-global singleton, so this can't
        // race the test below.
        let state = dummy_app_state();
        let starting = state.is_menu_open();
        let after_toggle = state.toggle_menu_open();
        assert_eq!(after_toggle, !starting);
        assert_eq!(state.is_menu_open(), !starting);
    }

    #[test]
    fn global_singleton_panics_before_init_then_returns_a_stable_instance_after() {
        // A `OnceLock` can't be reset, so one test owns the whole lifecycle:
        // panics before `init()`, stable instance after. Splitting caused
        // order-dependent races on the shared global.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence the expected-panic backtrace
        let before_init = std::panic::catch_unwind(get);
        std::panic::set_hook(previous_hook);
        assert!(before_init.is_err(), "get() must panic before init()");

        init(dummy_offsets(), Config::default(), SkinDatabase::empty());
        let a = std::ptr::from_ref(get());
        let b = std::ptr::from_ref(get());
        assert_eq!(a, b);
    }
}
