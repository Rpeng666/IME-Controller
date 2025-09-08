use crate::config::{ImeMode, CONFIG};
use log::info;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::{
    core::*,
    Win32::Foundation::{HWND, LPARAM, LRESULT, MAX_PATH, POINT, WPARAM},
};

pub fn handle_tray_message(hwnd: HWND, lparam: LPARAM) {
    match lparam.0 as u32 {
        WM_RBUTTONUP => show_tray_menu(hwnd),
        // WM_LBUTTONUP => show_tray_menu(hwnd), // 可以显示主窗口
        _ => {}
    }
}

// 辅助函数：向菜单添加项（宽字符版本）
fn append_menu_w(hmenu: HMENU, flags: MENU_ITEM_FLAGS, id: u32, text: &str) {
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        AppendMenuW(hmenu, flags, id as usize, PCWSTR(text_wide.as_ptr()));
    }
}

pub fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);

        let hmenu = CreatePopupMenu().unwrap();
        let config = CONFIG.read().unwrap();

        // 添加菜单项
        append_menu_w(
            hmenu,
            if config.master_switch {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
            crate::constants::IDM_MASTER_SWITCH,
            "启用软件功能",
        );

        append_menu_w(hmenu, MF_SEPARATOR, 0, "");

        append_menu_w(
            hmenu,
            if config.autostart {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
            crate::constants::IDM_AUTOSTART,
            "开机自动启动",
        );

        append_menu_w(
            hmenu,
            if config.show_notifications {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
            crate::constants::IDM_TOGGLE_NOTIFICATIONS,
            "显示通知",
        );

        // 输入法模式子菜单
        let mode_menu = CreatePopupMenu().unwrap();
        append_menu_w(
            mode_menu,
            if config.ime_mode == ImeMode::ChineseOnly {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
            crate::constants::IDM_IME_CHINESE,
            "强制中文模式",
        );
        append_menu_w(
            mode_menu,
            if config.ime_mode == ImeMode::EnglishOnly {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
            crate::constants::IDM_IME_ENGLISH,
            "强制英文模式",
        );
        append_menu_w(hmenu, MF_POPUP, mode_menu.0 as u32, "输入法模式");

        AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        append_menu_w(
            hmenu,
            MF_ENABLED,
            crate::constants::IDM_CONFIGURE_HOTKEYS,
            "查看快捷键设置",
        );
        append_menu_w(
            hmenu,
            MF_ENABLED,
            crate::constants::IDM_OPEN_CONFIG_DIR,
            "打开配置目录",
        );
        append_menu_w(
            hmenu,
            MF_ENABLED,
            crate::constants::IDM_RELOAD_CONFIG,
            "重新加载配置",
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        append_menu_w(hmenu, MF_ENABLED, crate::constants::IDM_EXIT, "退出");

        SetForegroundWindow(hwnd);
        TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, point.x, point.y, Some(0), hwnd, None);
        PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
}

pub fn handle_menu_command(hwnd: HWND, wparam: WPARAM) {
    info!("菜单命令: wparam = {}", wparam.0);
    let cmd = wparam.0 as u32;
    match cmd {
        crate::constants::IDM_EXIT => unsafe {
            info!("退出程序");
            PostQuitMessage(0);
        },
        crate::constants::IDM_AUTOSTART => {
            info!("切换开机自启动");
            crate::config::toggle_autostart(hwnd);
        }
        crate::constants::IDM_TOGGLE_NOTIFICATIONS => {
            info!("切换通知");
            crate::config::toggle_notifications(hwnd);
        }
        crate::constants::IDM_CONFIGURE_HOTKEYS => {
            info!("显示快捷键信息");
            crate::tray::notifications::show_hotkey_config_info(hwnd);
        }
        crate::constants::IDM_OPEN_CONFIG_DIR => {
            info!("打开配置目录");
            crate::config::open_config_directory();
        }
        crate::constants::IDM_RELOAD_CONFIG => {
            info!("重新加载配置");
            crate::config::reload_config_and_hotkeys(hwnd);
        }
        crate::constants::IDM_IME_CHINESE => {
            info!("切换输入法模式: 中文");
            crate::config::switch_ime_mode(hwnd, crate::config::ImeMode::ChineseOnly);
        }
        crate::constants::IDM_IME_ENGLISH => {
            info!("切换输入法模式: 英文");
            crate::config::switch_ime_mode(hwnd, crate::config::ImeMode::EnglishOnly);
        }
        crate::constants::IDM_MASTER_SWITCH => {
            info!("切换主开关");
            crate::config::toggle_master_switch(hwnd);
        }
        _ => {}
    }
}
