#![windows_subsystem = "windows"]

mod config;
mod logger;

use lazy_static::lazy_static;
use log::{debug, error, info};
use std::{sync::Mutex, thread, time::Duration};

use windows::{
    core::*,
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, MAX_PATH, POINT, WPARAM},
        Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
        System::{LibraryLoader::GetModuleHandleW, Threading::*},
        UI::{
            Accessibility::*,
            Input::{Ime::*, KeyboardAndMouse::*},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};

use winreg::enums::*;
use winreg::RegKey;

use crate::config::{Config, ImeMode, parse_hotkey};

// Constants
const IDI_ICON1: u16 = 101;
const IDI_ICON_ACTIVE: u16 = 102;
const IDM_EXIT: u32 = 1001;
const IDM_AUTOSTART: u32 = 1003;
const IDM_MANAGE_EXCLUDES: u32 = 1004;
const IDM_IME_CHINESE: u32 = 1005;
const IDM_IME_ENGLISH: u32 = 1006;
const IDM_MASTER_SWITCH: u32 = 1007;
const IDM_TOGGLE_NOTIFICATIONS: u32 = 1008;
const IDM_CONFIGURE_HOTKEYS: u32 = 1009;
const IDM_OPEN_CONFIG_DIR: u32 = 1010;
const IDM_RELOAD_CONFIG: u32 = 1011;
const NOTIFYICONMESSAGE: u32 = WM_USER + 100;
const HOT_KEY_TOGGLE_ID: i32 = 1; // 切换总开关
const HOT_KEY_SWITCH_MODE_ID: i32 = 2; // 切换中英文模式
const TIMER_ID_IME_CHECK: usize = 1; // 定时器ID，用于定时检测输入法状态

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

        if QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .as_bool()
        {
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

// windows事件触发回调函数
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

    // Get the keyboard layout name
    let mut buf = [0u8; 9];
    if GetKeyboardLayoutNameA(&mut buf).as_bool() {
        let lang_id = String::from_utf8_lossy(&buf);
        let lang_id_str = lang_id.trim_end_matches('\0');

        info!("Current keyboard layout: {}", lang_id_str);

        if lang_id_str == "00000804" {
            // 简体中文
            // Get the ime window handle
            let ime_hwnd = ImmGetDefaultIMEWnd(hwnd);
            if ime_hwnd.0 != 0 {
                // Sometimes the message will miss if we don't sleep for a little while.
                thread::sleep(Duration::from_millis(50));

                let target_mode = match config.ime_mode {
                    ImeMode::ChineseOnly => {
                        info!("Setting IME to Chinese mode");
                        1025 // IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE
                    }
                    ImeMode::EnglishOnly => {
                        info!("Setting IME to English mode");
                        0 // 完全关闭中文输入
                    }
                };

                let result = SendMessageW(
                    ime_hwnd,
                    WM_IME_CONTROL,
                    WPARAM(IMC_SETCONVERSIONMODE as usize),
                    LPARAM(target_mode),
                );
                
                info!("SendMessage result: {:?}, target_mode: {}", result, target_mode);
            } else {
                info!("IME window handle is null");
            }
        }
    }
}

// 手动应用输入法设置到当前窗口
fn apply_ime_setting_to_current_window() {
    unsafe {
        let current_hwnd = GetForegroundWindow();
        if current_hwnd.0 != 0 {
            event_hook_callback(
                HWINEVENTHOOK(0),
                EVENT_SYSTEM_FOREGROUND,
                current_hwnd,
                0,
                0,
                0,
                0,
            );
        }
    }
}

/// 检测并强制输入法状态 - 定时轮询版本
fn check_and_force_ime_state() {
    let config = CONFIG.lock().unwrap();
    
    // 如果总开关关闭，不做任何处理
    if !config.master_switch {
        return;
    }
    drop(config);
    
    unsafe {
        let current_hwnd = GetForegroundWindow();
        if current_hwnd.0 == 0 {
            return;
        }
        
        // 获取当前进程名，检查是否在排除列表中
        if let Some(process_name) = get_window_process_name(current_hwnd) {
            let config = CONFIG.lock().unwrap();
            if config.excluded_apps.contains(&process_name) {
                return;
            }
            drop(config);
        }
        
        // 直接调用现有的事件处理逻辑来强制切换
        event_hook_callback(
            HWINEVENTHOOK(0),
            EVENT_SYSTEM_FOREGROUND,
            current_hwnd,
            0,
            0,
            0,
            0,
        );
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
    let config = CONFIG.lock().unwrap();
    
    let icon_id = if config.master_switch { IDI_ICON_ACTIVE } else { IDI_ICON1 };
    let tooltip = if config.master_switch { "输入法控制 - 已启用" } else { "输入法控制 - 已禁用" };

    info!("config.master_switch: {}", config.master_switch);
    info!("Loading icon with ID: {}", icon_id);
    info!("Adding tray icon with tooltip: {}", tooltip);

    let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        hWnd: hwnd,
        uCallbackMessage: NOTIFYICONMESSAGE,
        hIcon: unsafe { LoadIconW(h_instance, PCWSTR(icon_id as *const u16))? },
        szTip: [0; 128],
        ..Default::default()
    };

    unsafe {
        for (i, c) in tooltip.encode_utf16().enumerate().take(127) {
            nid.szTip[i] = c;
        }
        Shell_NotifyIconW(NIM_ADD, &mut nid);
    }
    Ok(())
}

// 修改图标
fn update_tray_icon(hwnd: HWND, is_active: bool) -> windows::core::Result<()> {
    let h_instance = unsafe { GetModuleHandleW(None) }?;

    let icon_id = if is_active { IDI_ICON_ACTIVE } else { IDI_ICON1 };
    let tooltip = if is_active { "输入法控制 - 已启用" } else { "输入法控制 - 已禁用" };

    info!("Updating tray icon - is_active: {}, icon_id: {}", is_active, icon_id);

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uFlags: NIF_ICON | NIF_TIP,
        hIcon: unsafe { 
            let icon = LoadIconW(h_instance, PCWSTR(icon_id as *const u16))?;
            info!("Loaded icon handle: {:?}", icon);
            icon
        },
        szTip: [0; 128],
        ..Default::default()
    };

    unsafe {
        for (i, c) in tooltip.encode_utf16().enumerate().take(127) {
            nid.szTip[i] = c;
        }
        let result = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
        info!("Shell_NotifyIconW result: {:?}", result);
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
        // 处理托盘图标消息
        NOTIFYICONMESSAGE => match lparam.0 as u32 {
            // 鼠标左键点击, 显示或隐藏主窗口
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

                append_menu_w(
                    hmenu,
                    if config.show_notifications {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    IDM_TOGGLE_NOTIFICATIONS,
                    "显示通知",
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
                append_menu_w(hmenu, MF_ENABLED, IDM_CONFIGURE_HOTKEYS, "查看快捷键设置");
                append_menu_w(hmenu, MF_ENABLED, IDM_OPEN_CONFIG_DIR, "打开配置目录");
                append_menu_w(hmenu, MF_ENABLED, IDM_RELOAD_CONFIG, "重新加载配置");
                AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
                append_menu_w(hmenu, MF_ENABLED, IDM_EXIT, "退出");

                SetForegroundWindow(hwnd);
                TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, None);
                PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            }
            _ => {}
        },
        // 处理菜单项点击事件
        WM_COMMAND => match wparam.0 as u32 {
            // 推出
            IDM_EXIT => {
                PostQuitMessage(0);
            }
            // 处理菜单项点击事件
            IDM_AUTOSTART => {
                let mut config = CONFIG.lock().unwrap();
                config.autostart = !config.autostart;

                if let Ok(exe_path) = std::env::current_exe() {
                    let exe_path_str = exe_path.to_string_lossy();
                    
                    if config.autostart {
                        // 使用注册表设置开机自启动
                        info!("尝试设置开机自启动，路径: {}", exe_path_str);
                        match RegKey::predef(HKEY_CURRENT_USER)
                            .open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_WRITE)
                        {
                            Ok(run_key) => {
                                match run_key.set_value("IME Controller", &exe_path_str.as_ref()) {
                                    Ok(_) => {
                                        info!("开机自启动设置成功");
                                        show_balloon_tip(hwnd, "开机自启动", "已设置开机自动启动");
                                    },
                                    Err(e) => {
                                        error!("设置注册表值失败: {:?}", e);
                                        show_balloon_tip(hwnd, "错误", "设置开机自启动失败");
                                    },
                                }
                            }
                            Err(e) => {
                                error!("无法打开注册表Run键: {:?}", e);
                                show_balloon_tip(hwnd, "错误", "无法访问注册表");
                            },
                        }
                    } else {
                        // 删除注册表项
                        info!("尝试删除开机自启动项");
                        match RegKey::predef(HKEY_CURRENT_USER)
                            .open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_WRITE)
                        {
                            Ok(run_key) => {
                                match run_key.delete_value("IME Controller") {
                                    Ok(_) => {
                                        info!("开机自启动删除成功");
                                        show_balloon_tip(hwnd, "开机自启动", "已取消开机自动启动");
                                    },
                                    Err(e) => {
                                        error!("删除注册表值失败: {:?}", e);
                                        show_balloon_tip(hwnd, "提示", "开机自启动项不存在或已删除");
                                    },
                                }
                            }
                            Err(e) => {
                                error!("无法打开注册表Run键: {:?}", e);
                                show_balloon_tip(hwnd, "错误", "无法访问注册表");
                            },
                        }
                    }
                }

                config.save().ok();
            }
            // 切换通知开关
            IDM_TOGGLE_NOTIFICATIONS => {
                let mut config = CONFIG.lock().unwrap();
                config.show_notifications = !config.show_notifications;
                let notifications_enabled = config.show_notifications;
                config.save().ok();
                drop(config);
                
                // 只在开启通知时显示这个通知
                if notifications_enabled {
                    show_balloon_tip(hwnd, "设置", "已开启通知显示");
                }
            }
            // 配置快捷键
            IDM_CONFIGURE_HOTKEYS => {
                show_hotkey_config_info(hwnd);
            }
            // 打开配置目录
            IDM_OPEN_CONFIG_DIR => {
                open_config_directory();
            }
            // 重新加载配置
            IDM_RELOAD_CONFIG => {
                reload_config_and_hotkeys(hwnd);
                show_balloon_tip(hwnd, "配置重载", "配置已重新加载，热键已更新");
            }
            // 管理排除应用(暂时没啥用)
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
            // 切换输入法模式(强制中文)
            IDM_IME_CHINESE => {
                let mut config = CONFIG.lock().unwrap();
                config.ime_mode = ImeMode::ChineseOnly;
                config.save().ok();
                drop(config); // 释放锁
                
                // 立即应用到当前窗口
                apply_ime_setting_to_current_window();
                
                // show_balloon_tip(hwnd, "模式更改", "已切换到强制中文模式");
            }
            // 切换输入法模式(强制英文)
            IDM_IME_ENGLISH => {
                let mut config = CONFIG.lock().unwrap();
                config.ime_mode = ImeMode::EnglishOnly;
                config.save().ok();
                drop(config); // 释放锁
                
                // 立即应用到当前窗口
                apply_ime_setting_to_current_window();
                
                // show_balloon_tip(hwnd, "模式更改", "已切换到强制英文模式");
            }
            // 切换总开关
            IDM_MASTER_SWITCH => {
                let mut config = CONFIG.lock().unwrap();
                config.master_switch = !config.master_switch;
                let new_state = config.master_switch;
                config.save().ok();
                
                info!("Master switch changed to: {}", new_state);
                
                // 更新托盘图标
                match update_tray_icon(hwnd, new_state) {
                    Ok(_) => info!("Tray icon updated successfully"),
                    Err(e) => error!("Failed to update tray icon: {:?}", e),
                }
                
                if config.show_notifications {
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
            
            _ => {}
        },
        // 处理热键事件
        WM_HOTKEY => {
            match wparam.0 as i32 {
                HOT_KEY_TOGGLE_ID => {
                    // 切换总开关
                    let mut config = CONFIG.lock().unwrap();
                    config.master_switch = !config.master_switch;
                    let new_state = config.master_switch;
                    config.save().ok();
                    drop(config);
                    
                    // 更新托盘图标
                    let _ = update_tray_icon(hwnd, new_state);
                    
                    let config = CONFIG.lock().unwrap();
                    if config.show_notifications {
                        show_balloon_tip(
                            hwnd,
                            "快捷键触发",
                            if new_state {
                                "已启用强制输入法模式"
                            } else {
                                "已恢复输入法自动切换"
                            },
                        );
                    }
                }
                HOT_KEY_SWITCH_MODE_ID => {
                    // 切换中英文模式
                    let mut config = CONFIG.lock().unwrap();
                    config.ime_mode = match config.ime_mode {
                        ImeMode::ChineseOnly => ImeMode::EnglishOnly,
                        ImeMode::EnglishOnly => ImeMode::ChineseOnly,
                    };
                    let new_mode = config.ime_mode.clone();
                    config.save().ok();
                    drop(config);
                    
                    // 立即应用到当前窗口
                    apply_ime_setting_to_current_window();
                    
                    let mode_name = match new_mode {
                        ImeMode::ChineseOnly => "中文",
                        ImeMode::EnglishOnly => "英文",
                    };
                    info!("Switched to {} mode via hotkey", mode_name);
                }
                _ => {}
            }
        },
        // 处理定时器消息 - 定时检测输入法状态
        WM_TIMER => {
            if wparam.0 == TIMER_ID_IME_CHECK {
                // 执行定时检测输入法状态
                check_and_force_ime_state();
            }
        },
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

fn register_hotkey(hwnd: HWND, id: i32, modifiers: u32, key: u32) -> bool {
    unsafe { RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(modifiers), key).as_bool() }
}

fn unregister_hotkey(hwnd: HWND, id: i32) -> bool {
    unsafe { UnregisterHotKey(hwnd, id).as_bool() }
}

fn register_all_hotkeys(hwnd: HWND) {
    register_all_hotkeys_with_retry(hwnd, 3);
}

fn register_all_hotkeys_with_retry(hwnd: HWND, max_retries: i32) {
    let config = CONFIG.lock().unwrap();
    let toggle_hotkey = config.hotkey_toggle.clone();
    let switch_hotkey = config.hotkey_switch_mode.clone();
    drop(config);
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    // 注册切换总开关热键
    if let Some(hotkey_str) = &toggle_hotkey {
        total_count += 1;
        if let Some((modifiers, key)) = parse_hotkey(hotkey_str) {
            if register_hotkey(hwnd, HOT_KEY_TOGGLE_ID, modifiers, key) {
                info!("Registered toggle hotkey: {}", hotkey_str);
                success_count += 1;
            } else {
                error!("Failed to register toggle hotkey: {} (will retry)", hotkey_str);
            }
        } else {
            error!("Invalid hotkey format: {}", hotkey_str);
        }
    }
    
    // 注册切换到中英文模式热键
    if let Some(hotkey_str) = &switch_hotkey {
        total_count += 1;
        if let Some((modifiers, key)) = parse_hotkey(hotkey_str) {
            if register_hotkey(hwnd, HOT_KEY_SWITCH_MODE_ID, modifiers, key) {
                info!("Registered switch mode hotkey: {}", hotkey_str);
                success_count += 1;
            } else {
                error!("Failed to register switch mode hotkey: {} (will retry)", hotkey_str);
            }
        } else {
            error!("Invalid hotkey format: {}", hotkey_str);
        }
    }
    
    // 如果有热键注册失败，且还有重试次数，则延迟后重试
    if success_count < total_count && max_retries > 0 {
        info!("Some hotkeys failed to register, retrying in 2 seconds... ({} retries left)", max_retries);
        std::thread::sleep(Duration::from_secs(2));
        
        // 先注销已注册的热键，然后重新注册
        unregister_all_hotkeys(hwnd);
        register_all_hotkeys_with_retry(hwnd, max_retries - 1);
    } else if success_count == total_count {
        info!("All hotkeys registered successfully");
    } else {
        error!("Failed to register some hotkeys after all retries");
    }
}

fn unregister_all_hotkeys(hwnd: HWND) {
    unregister_hotkey(hwnd, HOT_KEY_TOGGLE_ID);
    unregister_hotkey(hwnd, HOT_KEY_SWITCH_MODE_ID);
}

// 打开配置文件所在目录
fn open_config_directory() {
    if let Some(config_dir) = dirs::config_dir() {
        let ime_config_dir = config_dir.join("ime-controller");
        if let Err(_) = std::fs::create_dir_all(&ime_config_dir) {
            return;
        }
        
        let dir_path = ime_config_dir.to_string_lossy();
        let command = format!("explorer \"{}\"", dir_path);
        
        std::process::Command::new("cmd")
            .args(&["/C", &command])
            .spawn()
            .ok();
    }
}

// 重新加载配置并重新注册热键
fn reload_config_and_hotkeys(hwnd: HWND) {
    // 先注销所有热键
    unregister_all_hotkeys(hwnd);
    
    // 重新加载配置
    let new_config = Config::load();
    {
        let mut config = CONFIG.lock().unwrap();
        *config = new_config;
    }
    
    // 重新注册热键
    register_all_hotkeys(hwnd);
    
    info!("Configuration reloaded and hotkeys re-registered");
}

// 检查开机自启动状态
fn check_autostart_status() -> bool {
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
    {
        Ok(run_key) => {
            match run_key.get_value::<String, _>("IME Controller") {
                Ok(value) => {
                    if let Ok(current_exe) = std::env::current_exe() {
                        let current_path = current_exe.to_string_lossy();
                        let registry_exists = !value.is_empty();
                        info!("注册表中的开机自启动项: {}", value);
                        info!("当前程序路径: {}", current_path);
                        registry_exists
                    } else {
                        false
                    }
                }
                Err(_) => {
                    info!("注册表中没有找到开机自启动项");
                    false
                }
            }
        }
        Err(e) => {
            error!("无法访问注册表Run键: {:?}", e);
            false
        }
    }
}

// 同步配置与注册表状态
fn sync_autostart_config() {
    let registry_autostart = check_autostart_status();
    let mut config = CONFIG.lock().unwrap();
    
    if config.autostart != registry_autostart {
        info!("同步开机自启动配置: 配置文件={}, 注册表={}", config.autostart, registry_autostart);
        config.autostart = registry_autostart;
        config.save().ok();
    }
}

// 检测是否可能是开机自启动
fn is_likely_autostart() -> bool {
    // 简单检测：检查程序是否通过注册表自启动方式运行
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
    {
        Ok(run_key) => {
            // 如果注册表中有自启动项，并且当前程序路径匹配，则认为是自启动
            if let Ok(value) = run_key.get_value::<String, _>("IME Controller") {
                if let Ok(current_exe) = std::env::current_exe() {
                    let current_path = current_exe.to_string_lossy();
                    let current_path_str = current_path.to_string();
                    value.contains(&current_path_str) || current_path_str.contains(&value)
                } else {
                    false
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

// 显示热键配置信息的函数
fn show_hotkey_config_info(hwnd: HWND) {
    let config = CONFIG.lock().unwrap();
    
    let toggle_key = config.hotkey_toggle
        .as_ref()
        .unwrap_or(&"未设置".to_string())
        .clone();
    
    let switch_mode_key = config.hotkey_switch_mode
        .as_ref()
        .unwrap_or(&"未设置".to_string())
        .clone();
    
    drop(config);
    
    let message = format!(
        "当前快捷键设置：\n\n\
        总开关切换: {}\n\
        中英文模式切换: {}\n\n\
        如需修改快捷键：\n\
        1. 右键托盘图标 → 打开配置目录\n\
        2. 编辑 config.json 文件\n\
        3. 右键托盘图标 → 重新加载配置\n\n\
        快捷键格式示例：\n\
        Alt+S, Ctrl+F, Shift+F1, Win+Space\n\
        支持的修饰键: Alt, Ctrl, Shift, Win\n\
        支持的按键: A-Z, F1-F12, Space, Enter, Esc, Tab",
        toggle_key, switch_mode_key
    );
    
    show_balloon_tip(hwnd, "快捷键配置帮助", &message);
}

fn main() -> windows::core::Result<()> {
    // 初始化日志系统
    if let Err(e) = logger::init_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    info!("Application starting...");

    // 检查是否是开机自启动（通过检查启动时间来判断）
    let is_autostart = is_likely_autostart();
    if is_autostart {
        info!("Detected autostart, waiting 5 seconds for system to fully load...");
        std::thread::sleep(Duration::from_secs(5));
    }

    // 同步开机自启动配置
    sync_autostart_config();

    // 创建主窗口
    let hwnd = create_window()?;

    // 添加托盘图标
    add_tray_icon(hwnd)?;

    // 注册热键（开机自启动时延迟注册）
    if is_autostart {
        info!("Registering hotkeys with retry mechanism for autostart...");
    }
    register_all_hotkeys(hwnd);

    // 设置窗口钩子
    let hook = set_window_hook();
    if hook.is_none() {
        error!("Failed to set window hook");
        return Err(Error::from_win32());
    }

    // 设置定时器，用于定时检测输入法状态
    unsafe {
        if SetTimer(hwnd, TIMER_ID_IME_CHECK, 500, None) == 0 { // 每500ms检测一次
            error!("Failed to set IME check timer");
            return Err(Error::from_win32());
        } else {
            info!("IME check timer set successfully (interval: 500ms)");
        }
    }

    // 消息循环
    create_message_loop();

    // 清理定时器和钩子
    unsafe {
        KillTimer(hwnd, TIMER_ID_IME_CHECK);
    }
    
    if let Some(hook) = hook {
        unsafe {
            UnhookWinEvent(hook);
        }
    }

    info!("Application exiting...");
    Ok(())
}
