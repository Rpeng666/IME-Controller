use crate::hooks::event_loop_hook::enforce_global_ime_mode;
use crate::tray::icon::update_tray_icon;
use crate::tray::notifications::show_balloon_tip;
use crate::utils::hot_key::register_all_hotkeys;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use winreg::enums::*;
use winreg::RegKey;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ImeMode {
    ChineseOnly, // 仅中文模式
    EnglishOnly, // 仅英文模式
}

impl Default for ImeMode {
    fn default() -> Self {
        ImeMode::ChineseOnly
    }
}

// Global state
lazy_static! {
    pub static ref CONFIG: RwLock<Config> = RwLock::new(Config::load());

    static ref CONFIG_PATH: PathBuf = {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // 去掉 exe 文件名
        path.push("config.json");
        path
    };
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub autostart: bool,
    pub excluded_apps: Vec<String>,
    pub hotkey_toggle: Option<String>, // 切换总开关快捷键 (如: "Alt+M")
    pub hotkey_switch_mode: Option<String>, // 切换中英文模式快捷键 (如: "Alt+S")
    pub ime_mode: ImeMode,             // 输入法模式
    pub master_switch: bool,           // 总开关
    pub show_notifications: bool,      // 是否显示通知
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: false,
            excluded_apps: vec!["ime-controller.exe".to_string()],
            hotkey_toggle: Some("Alt+M".to_string()), // Alt+M (总开关)
            hotkey_switch_mode: Some("CAPSLOCK".to_string()), // CAPSLOCK (切换中英文模式)
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
            info!("加载配置文件: {:?}", content);
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            info!("配置文件不存在，使用默认配置");
            let config = Config::default();
            if let Err(e) = config.save() {
                error!("保存默认配置文件失败: {}", e);
            }
            config
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::get_config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(config_path, json)
    }

    fn get_config_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::current_exe().unwrap().parent().unwrap());
        info!("配置文件目录: {:?}", path);
        path.push("config.json");
        path
    }
}

/// 切换总开关
pub fn toggle_master_switch(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.master_switch = !config.master_switch;
    let new_state = config.master_switch;
    config.save().ok();
    drop(config);

    let _ = update_tray_icon(hwnd, new_state);

    if CONFIG.read().unwrap().show_notifications {
        show_balloon_tip(
            hwnd,
            "状态更改",
            if new_state {
                "已启用强制输入法模式"
            } else {
                "已恢复输入法自动切换"
            },
        );
    }
}

/// 切换自动启动
pub fn toggle_autostart(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.autostart = !config.autostart;

    if let Ok(exe_path) = std::env::current_exe() {
        let exe_path_str = exe_path.to_string_lossy();
        match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE,
        ) {
            Ok(run_key) => {
                if config.autostart {
                    let _ = run_key.set_value("IME Controller", &exe_path_str.as_ref());
                    show_balloon_tip(hwnd, "开机自启动", "已设置开机自动启动");
                } else {
                    let _ = run_key.delete_value("IME Controller");
                    show_balloon_tip(hwnd, "开机自启动", "已取消开机自启动");
                }
            }
            Err(_) => {
                show_balloon_tip(hwnd, "错误", "无法访问注册表Run键");
            }
        }
    }

    config.save().ok();
}

/// 切换通知显示
pub fn toggle_notifications(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.show_notifications = !config.show_notifications;
    config.save().ok();

    if config.show_notifications {
        show_balloon_tip(hwnd, "设置", "已开启通知显示");
    }
}

/// 切换输入法模式
pub fn switch_ime_mode(hwnd: HWND, mode: ImeMode) {
    let mut config = CONFIG.write().unwrap();
    config.ime_mode = mode.clone();
    config.save().ok();
    drop(config);

    apply_ime_setting_to_current_window(hwnd, mode.clone());

    show_balloon_tip(
        hwnd,
        "模式更改",
        match mode {
            ImeMode::ChineseOnly => "已切换到强制中文模式",
            ImeMode::EnglishOnly => "已切换到强制英文模式",
        },
    );
}

/// 应用输入法设置到当前窗口
pub fn apply_ime_setting_to_current_window(hwnd: HWND, config: ImeMode) {
    unsafe {
        enforce_global_ime_mode(hwnd, &config);
    }
}

/// 打开配置目录
pub fn open_config_directory() {
    let config_dir = CONFIG_PATH.parent().unwrap();
    if let Err(e) = open::that(config_dir) {
        eprintln!("无法打开配置目录: {:?}", e);
    }
}

/// 重新加载配置并刷新热键
pub fn reload_config_and_hotkeys(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    *config = Config::load();
    drop(config);
    register_all_hotkeys(hwnd);
    info!("配置重新加载完成");
}
