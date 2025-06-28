use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// 热键解析功能
pub fn parse_hotkey(hotkey_str: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = hotkey_str.split('+').collect();
    if parts.len() < 2 {
        return None;
    }

    let mut modifiers = 0u32;
    let mut key_code = 0u32;

    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        if i == parts.len() - 1 {
            // 最后一个是按键
            key_code = match part.to_uppercase().as_str() {
                "A" => 65, "B" => 66, "C" => 67, "D" => 68, "E" => 69, "F" => 70,
                "G" => 71, "H" => 72, "I" => 73, "J" => 74, "K" => 75, "L" => 76,
                "M" => 77, "N" => 78, "O" => 79, "P" => 80, "Q" => 81, "R" => 82,
                "S" => 83, "T" => 84, "U" => 85, "V" => 86, "W" => 87, "X" => 88,
                "Y" => 89, "Z" => 90,
                "F1" => 0x70, "F2" => 0x71, "F3" => 0x72, "F4" => 0x73,
                "F5" => 0x74, "F6" => 0x75, "F7" => 0x76, "F8" => 0x77,
                "F9" => 0x78, "F10" => 0x79, "F11" => 0x7A, "F12" => 0x7B,
                "SPACE" => 0x20, "ENTER" => 0x0D, "ESC" => 0x1B,
                "TAB" => 0x09, "BACKSPACE" => 0x08,
                _ => return None,
            };
        } else {
            // 修饰键
            match part.to_uppercase().as_str() {
                "CTRL" | "CONTROL" => modifiers |= 0x0002, // MOD_CONTROL
                "ALT" => modifiers |= 0x0001, // MOD_ALT
                "SHIFT" => modifiers |= 0x0004, // MOD_SHIFT
                "WIN" | "WINDOWS" => modifiers |= 0x0008, // MOD_WIN
                _ => return None,
            }
        }
    }

    if key_code == 0 {
        None
    } else {
        Some((modifiers, key_code))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ImeMode {
    ChineseOnly, // 仅中文模式
    EnglishOnly, // 仅英文模式
}

impl Default for ImeMode {
    fn default() -> Self {
        ImeMode::ChineseOnly
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub enabled: bool,
    pub autostart: bool,
    pub excluded_apps: Vec<String>,
    pub hotkey_toggle: Option<String>,   // 切换总开关快捷键 (如: "Alt+M")
    pub hotkey_switch_mode: Option<String>, // 切换中英文模式快捷键 (如: "Alt+S")
    pub ime_mode: ImeMode,                 // 输入法模式
    pub master_switch: bool,               // 总开关
    pub show_notifications: bool,          // 是否显示通知
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: false,
            excluded_apps: vec!["ime-controller.exe".to_string()],
            hotkey_toggle: Some("Alt+M".to_string()), // Alt+M (总开关)
            hotkey_switch_mode: Some("Alt+S".to_string()), // Alt+S (切换中英文模式)
            ime_mode: ImeMode::default(),
            master_switch: true,       // 默认总开关为开启
            show_notifications: false, // 默认不显示通知，避免打扰
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::get_config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(config_path, json)
    }

    fn get_config_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::current_exe().unwrap().parent().unwrap());
        path.push("config.json");
        path
    }
}
