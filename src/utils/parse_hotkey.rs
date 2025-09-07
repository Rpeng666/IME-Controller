use log::info;

// 热键解析功能
pub fn parse_hotkey(hotkey_str: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = hotkey_str.split('+').collect();
    info!("发现hot key : {:?}", parts);

    // if parts.len() < 2 {
    //     return None;
    // }

    let mut modifiers = 0u32;
    let mut key_code = 0u32;

    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        if i == parts.len() - 1 {
            // 最后一个是按键
            key_code = match part.to_uppercase().as_str() {
                "A" => 65,
                "B" => 66,
                "C" => 67,
                "D" => 68,
                "E" => 69,
                "F" => 70,
                "G" => 71,
                "H" => 72,
                "I" => 73,
                "J" => 74,
                "K" => 75,
                "L" => 76,
                "M" => 77,
                "N" => 78,
                "O" => 79,
                "P" => 80,
                "Q" => 81,
                "R" => 82,
                "S" => 83,
                "T" => 84,
                "U" => 85,
                "V" => 86,
                "W" => 87,
                "X" => 88,
                "Y" => 89,
                "Z" => 90,
                "F1" => 0x70,
                "F2" => 0x71,
                "F3" => 0x72,
                "F4" => 0x73,
                "F5" => 0x74,
                "F6" => 0x75,
                "F7" => 0x76,
                "F8" => 0x77,
                "F9" => 0x78,
                "F10" => 0x79,
                "F11" => 0x7A,
                "F12" => 0x7B,
                "SPACE" => 0x20,
                "ENTER" => 0x0D,
                "ESC" => 0x1B,
                "TAB" => 0x09,
                "BACKSPACE" => 0x08,
                "CAPSLOCK" => 0x14,
                _ => return None,
            };
        } else {
            // 修饰键
            match part.to_uppercase().as_str() {
                "CTRL" | "CONTROL" => modifiers |= 0x0002, // MOD_CONTROL
                "ALT" => modifiers |= 0x0001,              // MOD_ALT
                "SHIFT" => modifiers |= 0x0004,            // MOD_SHIFT
                "WIN" | "WINDOWS" => modifiers |= 0x0008,  // MOD_WIN
                _ => return None,
            }
        }
    }
    info!("解析热键: modifiers = {}, key_code = {}", modifiers, key_code);  
    if key_code == 0 {
        None
    } else {
        Some((modifiers, key_code))
    }
}