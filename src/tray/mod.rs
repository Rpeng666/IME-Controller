pub mod create_window;
pub mod hotkey;
pub mod icon;
pub mod menu;
pub mod notifications;

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::tray::hotkey::*;
use crate::tray::menu::*;
use log::info;

pub const NOTIFYICON_MESSAGE: u32 = WM_USER + 100;

pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // info!("窗口消息: msg = {}, wparam = {}, lparam = {}", msg, wparam.0, lparam.0);
    match msg {
        NOTIFYICON_MESSAGE => handle_tray_message(hwnd, lparam),
        WM_COMMAND => handle_menu_command(hwnd, wparam),
        WM_HOTKEY => handle_hotkey(hwnd, wparam),
        WM_DESTROY => {
            info!("窗口销毁，退出消息循环");
            PostQuitMessage(0);
        }
        _ => {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
    }
    LRESULT(0)
}
