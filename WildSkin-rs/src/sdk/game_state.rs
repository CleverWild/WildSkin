pub const GAME_STATE_RUNNING: i32 = 2;

crate::offset!(
    pub struct GameClient {
        0x10 => game_state: i32,
    }
);

pub const unsafe fn is_running(client: *const GameClient) -> bool {
    if client.is_null() {
        return false;
    }
    // SAFETY: caller guarantees `client` is a live `GameClient` pointer
    // (checked non-null above).
    unsafe { (*client).game_state == GAME_STATE_RUNNING }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_running_state() {
        let client = GameClient { _padgame_state: [0; 0x10], game_state: GAME_STATE_RUNNING };
        unsafe { assert!(is_running(&raw const client)) };
    }

    #[test]
    fn rejects_non_running_states() {
        let client = GameClient { _padgame_state: [0; 0x10], game_state: 0 }; // LoadingScreen
        unsafe { assert!(!is_running(&raw const client)) };
    }

    #[test]
    fn null_pointer_is_not_running() {
        unsafe { assert!(!is_running(std::ptr::null())) };
    }
}
