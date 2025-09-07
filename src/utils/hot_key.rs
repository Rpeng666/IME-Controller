use crate::config;
use crate::constants;
use crate::utils::parse_hotkey::parse_hotkey;
use log::{error, info};
use std::time::Duration;
use windows::Win32::Foundation::{BOOL, HWND, GetLastError};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

// 注册所有热键，失败时重试
pub fn register_all_hotkeys(hwnd: HWND) {
    register_all_hotkeys_with_retry(hwnd, 3);
}

// 注册所有热键，失败时重试
fn register_all_hotkeys_with_retry(hwnd: HWND, max_retries: i32) {
    let config = config::CONFIG.read().unwrap();
    let toggle_hotkey = config.hotkey_toggle.clone();
    let switch_hotkey = config.hotkey_switch_mode.clone();
    drop(config);

    let mut success_count = 0;
    let mut total_count = 0;

    // 注册切换总开关热键
    if let Some(hotkey_str) = &toggle_hotkey {
        total_count += 1;
        if let Some((modifiers, key)) = parse_hotkey(hotkey_str) {
            if register_hotkey(hwnd, constants::HOT_KEY_TOGGLE_ID, modifiers, key) {
                info!("注册切换总开关热键: {}", hotkey_str);
                success_count += 1;
            } else {
                error!("注册失败: {} (will retry)", hotkey_str);
            }
        } else {
            error!("toggle_hotkey无效的热键格式: {}", hotkey_str);
        }
    }

    // 注册切换到中英文模式热键
    if let Some(hotkey_str) = &switch_hotkey {
        total_count += 1;
        if let Some((modifiers, key)) = parse_hotkey(hotkey_str) {
            if register_hotkey(hwnd, constants::HOT_KEY_SWITCH_MODE_ID, modifiers, key) {
                info!("注册语言切换热键成功: {}", hotkey_str);
                success_count += 1;
            } else {
                error!("注册语言切换热键失败: {} (will retry)", hotkey_str);
            }
        } else {
            error!("switch_hotkey无效的热键格式: {}", hotkey_str);
        }
    }

    // 如果有热键注册失败，且还有重试次数，则延迟后重试
    if success_count < total_count && max_retries > 0 {
        info!("有热键注册失败，2秒后重试... (剩余 {} 次重试)", max_retries);
        std::thread::sleep(Duration::from_secs(2));

        // 先注销已注册的热键，然后重新注册
        unregister_all_hotkeys(hwnd);
        register_all_hotkeys_with_retry(hwnd, max_retries - 1);
    } else if success_count == total_count {
        info!("所有的热键注册成功");
    } else {
        error!("所有热键注册失败");
    }
}

// 注册和注销热键
fn register_hotkey(hwnd: HWND, id: i32, modifiers: u32, key: u32) -> bool {
    unsafe {
        if RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(modifiers), key).as_bool() {
            true
        } else {
            let err_code = GetLastError().0;
            let err_msg = match err_code {
                1409 => "热键已被注册".to_string(),
                1408 => "尝试注销一个未注册的热键或ID不合法".to_string(),
                _ => format!("未知错误"),
            };
            error!("注册热键失败，错误码: {}，原因: {}", err_code, err_msg);
            false
        }
    }
}

// 注销热键
fn unregister_hotkey(hwnd: HWND, id: i32) -> bool {
    unsafe { UnregisterHotKey(hwnd, id).as_bool() }
}

// 注销所有热键
fn unregister_all_hotkeys(hwnd: HWND) {
    unregister_hotkey(hwnd, constants::HOT_KEY_TOGGLE_ID);
    unregister_hotkey(hwnd, constants::HOT_KEY_SWITCH_MODE_ID);
}
