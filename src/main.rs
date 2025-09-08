#![windows_subsystem = "windows"]

mod config;
mod constants;
mod hooks;
mod logger;
mod tray;
mod utils;

use log::{error, info};
use std::time::Duration;

use windows::Win32::{
    Foundation::HWND,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

use crate::create_window::create_window;
use crate::hooks::event_loop_hook::event_hook_callback;
use crate::icon::add_tray_icon;
use crate::tray::window_proc;
use crate::tray::*;
use crate::utils::check_autostart_status::is_likely_autostart;
use crate::utils::check_autostart_status::sync_autostart_config;
use crate::utils::hot_key::register_all_hotkeys;

// 设置Windows事件钩子
fn set_window_hook() -> Option<HWINEVENTHOOK> {
    unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,   // 监听的事件范围起点（这里是前台窗口切换）
            EVENT_SYSTEM_FOREGROUND,   // 监听的事件范围终点（同上）
            None,                      // DLL 句柄（一般 DLL 插件才用，EXE 里用 None）
            Some(event_hook_callback), // 回调函数
            0,                         // 进程 ID（0 = 全部进程）
            0,                         // 线程 ID（0 = 全部线程）
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS, // 钩子标志
        );

        if hook.is_invalid() {
            // 如果返回句柄为 0，说明设置失败
            None
        } else {
            Some(hook) // 成功就返回 hook 句柄
        }
    }
}

// 创建消息循环
fn create_message_loop() {
    unsafe {
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn main() -> windows::core::Result<()> {
    // 初始化日志系统
    if let Err(e) = logger::init_logger() {
        eprintln!("初始化日志系统失败: {}", e);
    }

    info!("系统启动中...");

    // 检查是否是开机自启动（通过检查启动时间来判断）
    let is_autostart = is_likely_autostart();
    if is_autostart {
        info!("检测到开机自启动，等待 8 秒以便系统完全加载...");
        std::thread::sleep(Duration::from_secs(8));
    }

    // 同步开机自启动配置
    sync_autostart_config();

    info!("创建主窗口...");
    // 创建主窗口
    let hwnd = create_window(window_proc)?;
    info!("主窗口创建成功，HWND={:?}", hwnd);

    info!("添加托盘图标...");
    // 添加托盘图标
    add_tray_icon(hwnd)?;
    info!("托盘图标添加成功");

    // 注册热键（开机自启动时延迟注册）
    if is_autostart {
        info!("为开机自启动注册热键，带重试机制...");
    }

    register_all_hotkeys(hwnd);
    info!("热键注册完成");

    // 设置窗口钩子
    let hook = set_window_hook().expect("钩子注册失败");

    // 消息循环
    create_message_loop();

    // 清理定时器和钩子
    unsafe {
        KillTimer(Some(hwnd), constants::TIMER_ID_IME_CHECK);
    }

    unsafe {
        UnhookWinEvent(hook);
    }

    info!("应用退出中...");
    Ok(())
}
