use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

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
    pub hotkey: Option<(u32, u32)>, // (modifiers, key)
    pub ime_mode: ImeMode,          // 输入法模式
    pub master_switch: bool,        // 总开关
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: false,
            excluded_apps: vec!["ime-controller.exe".to_string()],
            hotkey: Some((MOD_ALT.0 | MOD_CONTROL.0, 'M' as u32)),
            ime_mode: ImeMode::default(),
            master_switch: true, // 默认总开关为开启
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
