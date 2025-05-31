#![windows_subsystem = "windows"]

mod config;
mod logger;

use lazy_static::lazy_static;
use log::{error, info};
use std::{sync::Mutex, thread, time::Duration};
use std::os::windows::fs::symlink_file;

use windows::{
    core::*,
    Win32::{
        Foundation::{ BOOL, HWND, LPARAM, LRESULT, MAX_PATH, POINT, WPARAM },
        Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::*,
        },
        UI::{
            Accessibility::*,
            Input::{Ime::*, KeyboardAndMouse::*},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};

use crate::config::Config;
use crate::config::ImeMode;

// Constants
const IDI_ICON1: u16 = 101;
const IDM_EXIT: u32 = 1001;
const IDM_AUTOSTART: u32 = 1003;
const IDM_MANAGE_EXCLUDES: u32 = 1004;
const IDM_IME_CHINESE: u32 = 1005;
const IDM_IME_ENGLISH: u32 = 1006;
const IDM_MASTER_SWITCH: u32 = 1007;
const NOTIFYICONMESSAGE: u32 = WM_USER + 100;
const HOT_KEY_ID: i32 = 1;

// Global state
lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::load());
}

fn get_window_process_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL(0), process_id).ok()?;
        let mut buffer = [0u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;

        if QueryFullProcessImageNameW(process, PROCESS_NAME_FORMAT(0), PWSTR(buffer.as_mut_ptr()), &mut size).as_bool() {
            let name = String::from_utf16_lossy(&buffer[..size as usize]);
            let name = std::path::Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            return name;
        }
        None
    }
}

unsafe extern "system" fn event_hook_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let config = CONFIG.lock().unwrap();
    if !config.master_switch {
        return;
    }

    // Check if the current window is in excluded list
    if let Some(process_name) = get_window_process_name(hwnd) {
        if config.excluded_apps.contains(&process_name) {
            return;
        }
    }

    // Get the keyboard layout name
    let mut buf = [0u8; 9];
    if GetKeyboardLayoutNameA(&mut buf).as_bool() {
        let lang_id = String::from_utf8_lossy(&buf);
        let lang_id_str = lang_id.trim_end_matches('\0');
        
        if lang_id_str == "00000804" { // 简体中文
            // Get the ime window handle
            let ime_hwnd = ImmGetDefaultIMEWnd(hwnd);
            // Sometimes the message will miss if we don't sleep for a little while.
            thread::sleep(Duration::from_millis(50));

            match config.ime_mode {
                ImeMode::ChineseOnly => {
                    SendMessageW(
                        ime_hwnd,
                        WM_IME_CONTROL,
                        WPARAM(IMC_SETCONVERSIONMODE as usize),
                        LPARAM(1025), // Chinese
                    );
                }
                ImeMode::EnglishOnly => {
                    SendMessageW(
                        ime_hwnd,
                        WM_IME_CONTROL,
                        WPARAM(IMC_SETCONVERSIONMODE as usize),
                        LPARAM(0), // English
                    );
                }
            }
        }
    }
}

fn show_balloon_tip(hwnd: HWND, title: &str, message: &str) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_INFO,
        szInfoTitle: [0; 64],
        szInfo: [0; 256],
        ..Default::default()
    };

    for (i, c) in title.encode_utf16().take(63).enumerate() {
        nid.szInfoTitle[i] = c;
    }
    for (i, c) in message.encode_utf16().take(255).enumerate() {
        nid.szInfo[i] = c;
    }

    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &mut nid);
    }
}

fn add_tray_icon(hwnd: HWND) -> windows::core::Result<()> {
    let h_instance = unsafe { GetModuleHandleW(None) }?;

    let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        hWnd: hwnd,
        uCallbackMessage: NOTIFYICONMESSAGE,
        hIcon: unsafe { LoadIconW(h_instance, PCWSTR(IDI_ICON1 as *const u16))? },
        szTip: [0; 128],
        ..Default::default()
    };

    unsafe {
        let tooltip = "中文输入法模式\0";
        for (i, c) in tooltip.encode_utf16().enumerate() {
            nid.szTip[i] = c;
        }
        Shell_NotifyIconW(NIM_ADD, &mut nid);
    }
    Ok(())
}

fn append_menu_w(hmenu: HMENU, flags: MENU_ITEM_FLAGS, id: u32, text: &str) {
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        AppendMenuW(hmenu, flags, id as usize, PCWSTR(text_wide.as_ptr()));
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        NOTIFYICONMESSAGE => match lparam.0 as u32 {
            WM_RBUTTONUP => {
                let mut point = POINT::default();
                let _ = GetCursorPos(&mut point);

                let hmenu = CreatePopupMenu().unwrap();
                let config = CONFIG.lock().unwrap();

                // 添加总开关选项
                append_menu_w(
                    hmenu,
                    if config.master_switch {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    IDM_MASTER_SWITCH,
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
                    IDM_AUTOSTART,
                    "开机自动启动",
                );

                // 添加应用排除管理选项
                // append_menu_w(hmenu, MF_ENABLED, IDM_MANAGE_EXCLUDES, "管理排除应用");

                AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

                // 添加输入法模式子菜单
                let mode_menu = CreatePopupMenu().unwrap();
                append_menu_w(
                    mode_menu,
                    if config.ime_mode == ImeMode::ChineseOnly {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    IDM_IME_CHINESE,
                    "强制中文模式",
                );
                append_menu_w(
                    mode_menu,
                    if config.ime_mode == ImeMode::EnglishOnly {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    IDM_IME_ENGLISH,
                    "强制英文模式",
                );

                append_menu_w(hmenu, MF_POPUP, mode_menu.0 as u32, "输入法模式");

                AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
                append_menu_w(hmenu, MF_ENABLED, IDM_EXIT, "退出");

                SetForegroundWindow(hwnd);
                TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, None);
                PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            }
            _ => {}
        },
        WM_COMMAND => match wparam.0 as u32 {
            IDM_EXIT => {
                PostQuitMessage(0);
            }

            IDM_AUTOSTART => {
                let mut config = CONFIG.lock().unwrap();
                config.autostart = !config.autostart;
                
                if let Some(startup_dir) = dirs::config_dir().map(|d| d.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Startup")) {
                    if let Ok(exe_path) = std::env::current_exe() {
                        let startup_link = startup_dir.join("IME Controller.lnk");
                        
                        if config.autostart {
                            // 创建快捷方式
                            use std::os::windows::fs::symlink_file;
                            let _ = std::fs::create_dir_all(&startup_dir);
                            if symlink_file(&exe_path, &startup_link).is_ok() {
                                show_balloon_tip(
                                    hwnd,
                                    "开机自启动",
                                    "已设置开机自动启动",
                                );
                            }
                        } else {
                            // 删除快捷方式
                            if std::fs::remove_file(startup_link).is_ok() {
                                show_balloon_tip(
                                    hwnd,
                                    "开机自启动",
                                    "已取消开机自动启动",
                                );
                            }
                        }
                    }
                }
                
                config.save().ok();
            }
            IDM_MANAGE_EXCLUDES => {
                if let Some(curr_process) = get_window_process_name(GetForegroundWindow()) {
                    let mut config = CONFIG.lock().unwrap();
                    if !config.excluded_apps.contains(&curr_process) {
                        config.excluded_apps.push(curr_process.clone());
                        config.save().ok();
                        show_balloon_tip(
                            hwnd,
                            "已添加排除应用",
                            &format!("已将 {} 添加到排除列表", curr_process),
                        );
                    } else {
                        config.excluded_apps.retain(|x| x != &curr_process);
                        config.save().ok();
                        show_balloon_tip(
                            hwnd,
                            "已移除排除应用",
                            &format!("已将 {} 从排除列表中移除", curr_process),
                        );
                    }
                }
            }
            IDM_IME_CHINESE => {
                let mut config = CONFIG.lock().unwrap();
                config.ime_mode = ImeMode::ChineseOnly;
                config.save().ok();
                show_balloon_tip(hwnd, "模式更改", "已切换到强制中文模式");
            }
            IDM_IME_ENGLISH => {
                let mut config = CONFIG.lock().unwrap();
                config.ime_mode = ImeMode::EnglishOnly;
                config.save().ok();
                show_balloon_tip(hwnd, "模式更改", "已切换到强制英文模式");
            }
            IDM_MASTER_SWITCH => {
                let mut config = CONFIG.lock().unwrap();
                config.master_switch = !config.master_switch;
                config.save().ok();
                show_balloon_tip(
                    hwnd,
                    "状态更改",
                    if config.master_switch {
                        "已启用强制输入法模式"
                    } else {
                        "已恢复输入法自动切换"
                    },
                );
            }
            _ => {}
        },
        WM_HOTKEY => {
            if wparam.0 as i32 == HOT_KEY_ID {
                let mut config = CONFIG.lock().unwrap();
                config.enabled = !config.enabled;
                config.save().ok();
                show_balloon_tip(
                    hwnd,
                    "快捷键触发",
                    if config.enabled {
                        "已启用自动中文模式"
                    } else {
                        "已禁用自动中文模式"
                    },
                );
            }
        }
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }

    LRESULT(0)
}

fn create_window() -> Result<HWND> {
    let instance = unsafe { GetModuleHandleW(None)? };
    let window_class = w!("IME_CONTROL_WINDOW");
    let wc = WNDCLASSW {
        style: WNDCLASS_STYLES(CS_HREDRAW.0 | CS_VREDRAW.0),
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: unsafe { LoadIconW(instance, PCWSTR(IDI_ICON1 as *const u16))? },
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: window_class,
    };

    unsafe {
        RegisterClassW(&wc);

        // 创建一个隐藏的窗口
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW, // 使用工具窗口样式
            window_class,
            w!("输入法控制"),
            WS_OVERLAPPED, // 不使用 WS_VISIBLE
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1, // 最小化窗口大小
            1,
            None,
            None,
            instance,
            None,
        );

        if hwnd.0 == 0 {
            Err(Error::from_win32())
        } else {
            Ok(hwnd)
        }
    }
}

fn set_window_hook() -> Option<HWINEVENTHOOK> {
    unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(event_hook_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        
        if hook.0 == 0 {
            None
        } else {
            Some(hook)
        }
    }
}

fn create_message_loop() {
    unsafe {
        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND(0), 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn register_hotkey(hwnd: HWND, modifiers: u32, key: u32) -> bool {
    unsafe {
        RegisterHotKey(
            hwnd,
            HOT_KEY_ID,
            HOT_KEY_MODIFIERS(modifiers),
            key,
        ).as_bool()
    }
}

fn main() -> windows::core::Result<()> {
    // 初始化日志系统
    if let Err(e) = logger::init_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    info!("Application starting...");

    // 创建主窗口
    let hwnd = create_window()?;

    // 添加托盘图标
    add_tray_icon(hwnd)?;

    // 注册热键
    if let Some((modifiers, key)) = CONFIG.lock().unwrap().hotkey {
        register_hotkey(hwnd, modifiers, key);
    }

    // 设置窗口钩子
    let hook = set_window_hook();
    if hook.is_none() {
        error!("Failed to set window hook");
        return Err(Error::from_win32());
    }

    // 消息循环
    create_message_loop();

    // 清理钩子
    if let Some(hook) = hook {
        unsafe {
            UnhookWinEvent(hook);
        }
    }

    info!("Application exiting...");
    Ok(())
}
