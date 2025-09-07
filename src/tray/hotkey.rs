use crate::config::{apply_ime_setting_to_current_window, ImeMode, CONFIG};
use crate::constants;
use crate::tray::icon;
use crate::tray::notifications::show_balloon_tip;
use log::info;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

pub fn handle_hotkey(hwnd: HWND, wparam: WPARAM) {
    match wparam.0 as i32 {
        constants::HOT_KEY_TOGGLE_ID => {
            info!("热键触发: 切换总开关");
            let mut config = CONFIG.write().unwrap();
            config.master_switch = !config.master_switch;
            config.save().ok();

            drop(config);

            let _ = icon::update_tray_icon(hwnd, CONFIG.read().unwrap().master_switch);

            let msg = if CONFIG.read().unwrap().master_switch {
                "已启用强制输入法模式"
            } else {
                "已恢复输入法自动切换"
            };

            if CONFIG.read().unwrap().show_notifications {
                show_balloon_tip(hwnd, "快捷键触发", msg);
            }
        }
        constants::HOT_KEY_SWITCH_MODE_ID => {
            info!("热键触发: 切换输入法模式");
            let mut config = CONFIG.write().unwrap();
            let new_mode = match config.ime_mode {
                ImeMode::ChineseOnly => ImeMode::EnglishOnly,
                ImeMode::EnglishOnly => ImeMode::ChineseOnly,
            };
            config.ime_mode = new_mode.clone();
            config.save().ok();
            drop(config);
            apply_ime_setting_to_current_window(hwnd, new_mode);
        }
        _ => {}
    }
}
