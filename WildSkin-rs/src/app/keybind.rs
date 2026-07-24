// Full keyboard/mouse key enum. Most variants are built at runtime by
// `vk_to_code` via transmute from a bounds-checked index, invisible to
// dead-code analysis.
#[expect(
    dead_code,
    reason = "variants are built via transmute from a validated index, invisible to dead-code analysis"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyCode {
    Apostrophe,
    Comma,
    Minus,
    Period,
    Slash,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Semicolon,
    Equals,
    A,
    Add,
    B,
    Backspace,
    C,
    Capslock,
    D,
    Decimal,
    Delete,
    Divide,
    Down,
    E,
    End,
    Enter,
    F,
    F1,
    F10,
    F11,
    F12,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    G,
    H,
    Home,
    I,
    Insert,
    J,
    K,
    L,
    Lalt,
    Lctrl,
    Left,
    Lshift,
    M,
    Mouse1,
    Mouse2,
    Mouse3,
    Mouse4,
    Mouse5,
    Multiply,
    MousewheelDown,
    MousewheelUp,
    N,
    None,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    O,
    P,
    PageDown,
    PageUp,
    Q,
    R,
    Ralt,
    Rctrl,
    Right,
    Rshift,
    S,
    Space,
    Subtract,
    T,
    Tab,
    U,
    Up,
    V,
    W,
    X,
    Y,
    Z,
    Leftbracket,
    Backslash,
    Rightbracket,
    Backtick,
}

struct KeyEntry {
    name: &'static str,
    vk_code: i32,
}

pub struct KeyBind {
    code: KeyCode,
}

const KEY_TABLE: &[KeyEntry] = &[
    KeyEntry {
        name: "'",
        vk_code: 0xDE,
    }, // VK_OEM_7
    KeyEntry {
        name: ",",
        vk_code: 0xBC,
    }, // VK_OEM_COMMA
    KeyEntry {
        name: "-",
        vk_code: 0xBD,
    }, // VK_OEM_MINUS
    KeyEntry {
        name: ".",
        vk_code: 0xBE,
    }, // VK_OEM_PERIOD
    KeyEntry {
        name: "/",
        vk_code: 0xBF,
    }, // VK_OEM_2
    KeyEntry {
        name: "0",
        vk_code: b'0' as i32,
    },
    KeyEntry {
        name: "1",
        vk_code: b'1' as i32,
    },
    KeyEntry {
        name: "2",
        vk_code: b'2' as i32,
    },
    KeyEntry {
        name: "3",
        vk_code: b'3' as i32,
    },
    KeyEntry {
        name: "4",
        vk_code: b'4' as i32,
    },
    KeyEntry {
        name: "5",
        vk_code: b'5' as i32,
    },
    KeyEntry {
        name: "6",
        vk_code: b'6' as i32,
    },
    KeyEntry {
        name: "7",
        vk_code: b'7' as i32,
    },
    KeyEntry {
        name: "8",
        vk_code: b'8' as i32,
    },
    KeyEntry {
        name: "9",
        vk_code: b'9' as i32,
    },
    KeyEntry {
        name: ";",
        vk_code: 0xBA,
    }, // VK_OEM_1
    KeyEntry {
        name: "=",
        vk_code: 0xBB,
    }, // VK_OEM_PLUS
    KeyEntry {
        name: "A",
        vk_code: b'A' as i32,
    },
    KeyEntry {
        name: "ADD",
        vk_code: 0x6B,
    }, // VK_ADD
    KeyEntry {
        name: "B",
        vk_code: b'B' as i32,
    },
    KeyEntry {
        name: "BACKSPACE",
        vk_code: 0x08,
    }, // VK_BACK
    KeyEntry {
        name: "C",
        vk_code: b'C' as i32,
    },
    KeyEntry {
        name: "CAPSLOCK",
        vk_code: 0x14,
    }, // VK_CAPITAL
    KeyEntry {
        name: "D",
        vk_code: b'D' as i32,
    },
    KeyEntry {
        name: "DECIMAL",
        vk_code: 0x6E,
    }, // VK_DECIMAL
    KeyEntry {
        name: "DELETE",
        vk_code: 0x2E,
    }, // VK_DELETE
    KeyEntry {
        name: "DIVIDE",
        vk_code: 0x6F,
    }, // VK_DIVIDE
    KeyEntry {
        name: "DOWN",
        vk_code: 0x28,
    }, // VK_DOWN
    KeyEntry {
        name: "E",
        vk_code: b'E' as i32,
    },
    KeyEntry {
        name: "END",
        vk_code: 0x23,
    }, // VK_END
    KeyEntry {
        name: "ENTER",
        vk_code: 0x0D,
    }, // VK_RETURN
    KeyEntry {
        name: "F",
        vk_code: b'F' as i32,
    },
    KeyEntry {
        name: "F1",
        vk_code: 0x70,
    },
    KeyEntry {
        name: "F10",
        vk_code: 0x79,
    },
    KeyEntry {
        name: "F11",
        vk_code: 0x7A,
    },
    KeyEntry {
        name: "F12",
        vk_code: 0x7B,
    },
    KeyEntry {
        name: "F2",
        vk_code: 0x71,
    },
    KeyEntry {
        name: "F3",
        vk_code: 0x72,
    },
    KeyEntry {
        name: "F4",
        vk_code: 0x73,
    },
    KeyEntry {
        name: "F5",
        vk_code: 0x74,
    },
    KeyEntry {
        name: "F6",
        vk_code: 0x75,
    },
    KeyEntry {
        name: "F7",
        vk_code: 0x76,
    },
    KeyEntry {
        name: "F8",
        vk_code: 0x77,
    },
    KeyEntry {
        name: "F9",
        vk_code: 0x78,
    },
    KeyEntry {
        name: "G",
        vk_code: b'G' as i32,
    },
    KeyEntry {
        name: "H",
        vk_code: b'H' as i32,
    },
    KeyEntry {
        name: "HOME",
        vk_code: 0x24,
    }, // VK_HOME
    KeyEntry {
        name: "I",
        vk_code: b'I' as i32,
    },
    KeyEntry {
        name: "INSERT",
        vk_code: 0x2D,
    }, // VK_INSERT
    KeyEntry {
        name: "J",
        vk_code: b'J' as i32,
    },
    KeyEntry {
        name: "K",
        vk_code: b'K' as i32,
    },
    KeyEntry {
        name: "L",
        vk_code: b'L' as i32,
    },
    KeyEntry {
        name: "LALT",
        vk_code: 0xA4,
    }, // VK_LMENU
    KeyEntry {
        name: "LCTRL",
        vk_code: 0xA2,
    }, // VK_LCONTROL
    KeyEntry {
        name: "LEFT",
        vk_code: 0x25,
    }, // VK_LEFT
    KeyEntry {
        name: "LSHIFT",
        vk_code: 0xA0,
    }, // VK_LSHIFT
    KeyEntry {
        name: "M",
        vk_code: b'M' as i32,
    },
    KeyEntry {
        name: "MOUSE1",
        vk_code: 0x0,
    },
    KeyEntry {
        name: "MOUSE2",
        vk_code: 0x1,
    },
    KeyEntry {
        name: "MOUSE3",
        vk_code: 0x2,
    },
    KeyEntry {
        name: "MOUSE4",
        vk_code: 0x3,
    },
    KeyEntry {
        name: "MOUSE5",
        vk_code: 0x4,
    },
    KeyEntry {
        name: "MULTIPLY",
        vk_code: 0x6A,
    }, // VK_MULTIPLY
    KeyEntry {
        name: "MWHEEL_DOWN",
        vk_code: 0x0,
    },
    KeyEntry {
        name: "MWHEEL_UP",
        vk_code: 0x0,
    },
    KeyEntry {
        name: "N",
        vk_code: b'N' as i32,
    },
    KeyEntry {
        name: "NONE",
        vk_code: 0x0,
    },
    KeyEntry {
        name: "NUMPAD_0",
        vk_code: 0x60,
    },
    KeyEntry {
        name: "NUMPAD_1",
        vk_code: 0x61,
    },
    KeyEntry {
        name: "NUMPAD_2",
        vk_code: 0x62,
    },
    KeyEntry {
        name: "NUMPAD_3",
        vk_code: 0x63,
    },
    KeyEntry {
        name: "NUMPAD_4",
        vk_code: 0x64,
    },
    KeyEntry {
        name: "NUMPAD_5",
        vk_code: 0x65,
    },
    KeyEntry {
        name: "NUMPAD_6",
        vk_code: 0x66,
    },
    KeyEntry {
        name: "NUMPAD_7",
        vk_code: 0x67,
    },
    KeyEntry {
        name: "NUMPAD_8",
        vk_code: 0x68,
    },
    KeyEntry {
        name: "NUMPAD_9",
        vk_code: 0x69,
    },
    KeyEntry {
        name: "O",
        vk_code: b'O' as i32,
    },
    KeyEntry {
        name: "P",
        vk_code: b'P' as i32,
    },
    KeyEntry {
        name: "PAGE_DOWN",
        vk_code: 0x22,
    }, // VK_NEXT
    KeyEntry {
        name: "PAGE_UP",
        vk_code: 0x21,
    }, // VK_PRIOR
    KeyEntry {
        name: "Q",
        vk_code: b'Q' as i32,
    },
    KeyEntry {
        name: "R",
        vk_code: b'R' as i32,
    },
    KeyEntry {
        name: "RALT",
        vk_code: 0xA5,
    }, // VK_RMENU
    KeyEntry {
        name: "RCTRL",
        vk_code: 0xA3,
    }, // VK_RCONTROL
    KeyEntry {
        name: "RIGHT",
        vk_code: 0x27,
    }, // VK_RIGHT
    KeyEntry {
        name: "RSHIFT",
        vk_code: 0xA1,
    }, // VK_RSHIFT
    KeyEntry {
        name: "S",
        vk_code: b'S' as i32,
    },
    KeyEntry {
        name: "SPACE",
        vk_code: 0x20,
    }, // VK_SPACE
    KeyEntry {
        name: "SUBTRACT",
        vk_code: 0x6D,
    }, // VK_SUBTRACT
    KeyEntry {
        name: "T",
        vk_code: b'T' as i32,
    },
    KeyEntry {
        name: "TAB",
        vk_code: 0x09,
    }, // VK_TAB
    KeyEntry {
        name: "U",
        vk_code: b'U' as i32,
    },
    KeyEntry {
        name: "UP",
        vk_code: 0x26,
    }, // VK_UP
    KeyEntry {
        name: "V",
        vk_code: b'V' as i32,
    },
    KeyEntry {
        name: "W",
        vk_code: b'W' as i32,
    },
    KeyEntry {
        name: "X",
        vk_code: b'X' as i32,
    },
    KeyEntry {
        name: "Y",
        vk_code: b'Y' as i32,
    },
    KeyEntry {
        name: "Z",
        vk_code: b'Z' as i32,
    },
    KeyEntry {
        name: "[",
        vk_code: 0xDB,
    }, // VK_OEM_4
    KeyEntry {
        name: "\\",
        vk_code: 0xDC,
    }, // VK_OEM_5
    KeyEntry {
        name: "]",
        vk_code: 0xDD,
    }, // VK_OEM_6
    KeyEntry {
        name: "`",
        vk_code: 0xC0,
    }, // VK_OEM_3
];

impl KeyBind {
    pub const fn new(code: KeyCode) -> Self {
        Self { code }
    }

    pub fn from_name(name: &str) -> Self {
        KEY_TABLE.iter().position(|e| e.name == name).map_or(
            Self {
                code: KeyCode::None,
            },
            |idx| Self {
                code: index_to_code(idx),
            },
        )
    }

    pub fn to_string(&self) -> &'static str {
        KEY_TABLE[self.code as usize].name
    }

    pub fn vk_code(&self) -> i32 {
        KEY_TABLE[self.code as usize].vk_code
    }

    // Used by the not-yet-wired per-frame hotkey checks in gui.rs.
    #[allow(
        dead_code,
        reason = "consumed by the not-yet-wired per-frame hotkey checks in gui.rs"
    )]
    pub fn is_set(&self) -> bool {
        self.code != KeyCode::None
    }

    #[allow(
        dead_code,
        reason = "consumed by the not-yet-wired per-frame hotkey checks in gui.rs"
    )]
    pub const fn code(&self) -> KeyCode {
        self.code
    }
}

fn index_to_code(idx: usize) -> KeyCode {
    // SAFETY: KeyCode is #[repr(u8)] with variants in exactly KEY_TABLE's
    // order (0..KEY_TABLE.len()), so any in-range index is a valid discriminant.
    unsafe { std::mem::transmute(idx as u8) }
}

/// Reverse lookup: the `KeyCode` (if any) for this Win32 VK code. Used by
/// `set_to_pressed_key`; `pub(crate)` keeps `KEY_TABLE` private.
pub(crate) fn vk_to_code(vk: i32) -> Option<KeyCode> {
    KEY_TABLE
        .iter()
        .position(|e| e.vk_code == vk)
        .map(index_to_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_name() {
        let kb = KeyBind::from_name("INSERT");
        assert_eq!(kb.to_string(), "INSERT");
    }

    #[test]
    fn unknown_name_falls_back_to_none() {
        let kb = KeyBind::from_name("NOT_A_REAL_KEY");
        assert!(!kb.is_set());
    }

    #[test]
    fn insert_maps_to_the_win32_vk_code() {
        // VK_INSERT = 0x2D
        assert_eq!(KeyBind::from_name("INSERT").vk_code(), 0x2D);
    }
}
