

pub fn mouse_to_engin(key: u8) -> Option<enigo::MouseButton> {
    match key {
        0 => Some(enigo::MouseButton::Left),
        1 => Some(enigo::MouseButton::Middle),
        2 => Some(enigo::MouseButton::Right),
        _ => None,
    }
}

pub fn key_to_enigo(key: u8) -> Option<enigo::Key> {
    match key {
        27 => Some(enigo::Key::Escape),
        112 => Some(enigo::Key::F1),
        113 => Some(enigo::Key::F2),
        114 => Some(enigo::Key::F3),
        115 => Some(enigo::Key::F4),
        116 => Some(enigo::Key::F5),
        117 => Some(enigo::Key::F6),
        118 => Some(enigo::Key::F7),
        119 => Some(enigo::Key::F8),
        120 => Some(enigo::Key::F9),
        121 => Some(enigo::Key::F10),
        122 => Some(enigo::Key::F11),
        123 => Some(enigo::Key::F12),
        // 19 => Some(enigo::Key::Pause), // Pause
        // 97 => Some(enigo::Key::Print), // Print
        46 => Some(enigo::Key::Delete),
        35 => Some(enigo::Key::End),
        192 => Some(enigo::Key::Layout('`')),
        48 => Some(enigo::Key::Layout('0')),
        49 => Some(enigo::Key::Layout('1')),
        50 => Some(enigo::Key::Layout('2')),
        51 => Some(enigo::Key::Layout('3')),
        52 => Some(enigo::Key::Layout('4')),
        53 => Some(enigo::Key::Layout('5')),
        54 => Some(enigo::Key::Layout('6')),
        55 => Some(enigo::Key::Layout('7')),
        56 => Some(enigo::Key::Layout('8')),
        57 => Some(enigo::Key::Layout('9')),
        189 => Some(enigo::Key::Layout('-')),
        187 => Some(enigo::Key::Layout('=')),
        8 => Some(enigo::Key::Backspace),
        9 => Some(enigo::Key::Tab),
        219 => Some(enigo::Key::Layout('[')),
        221 => Some(enigo::Key::Layout(']')),
        220 => Some(enigo::Key::Layout('\\')),
        20 => Some(enigo::Key::CapsLock),
        186 => Some(enigo::Key::Layout(';')),
        222 => Some(enigo::Key::Layout('\'')),
        13 => Some(enigo::Key::Return),
        16 => Some(enigo::Key::Shift), // ShiftL
        188 => Some(enigo::Key::Layout(',')),
        190 => Some(enigo::Key::Layout('.')),
        191 => Some(enigo::Key::Layout('/')),
        161 => Some(enigo::Key::Shift), // ShiftR
        38 => Some(enigo::Key::UpArrow),
        17 => Some(enigo::Key::Control), // ControlL
        18 => Some(enigo::Key::Alt),     // AltL
        32 => Some(enigo::Key::Space),
        165 => Some(enigo::Key::Alt), // AltR
        // 103 => Some(enigo::Key::Menu),
        163 => Some(enigo::Key::Control), // ControlR
        37 => Some(enigo::Key::LeftArrow),
        40 => Some(enigo::Key::DownArrow),
        39 => Some(enigo::Key::RightArrow),
        // 99 => Some(enigo::Key::Raw(45)), // Insert
        34 => Some(enigo::Key::PageDown),
        36 => Some(enigo::Key::Home),
        33 => Some(enigo::Key::PageUp),
        a if a >= 65 && a <= 90 => Some(enigo::Key::Layout((a - 65 + ('a' as u8)) as char)),
        _ => None,
    }
}