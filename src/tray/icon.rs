use crate::constants;
use crate::config::CONFIG;
use log::info;
use windows::{
    core::*,
    Win32::{
        Foundation::HWND,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

pub fn update_tray_icon(hwnd: HWND, enabled: bool) -> windows::core::Result<()> {
    // 封装托盘图标更新逻辑
    let h_instance = unsafe { GetModuleHandleW(None) }?;

    let icon_id = if enabled {
        constants::IDI_ICON_ACTIVE
    } else {
        constants::IDI_ICON1
    };
    let tooltip = if enabled {
        "输入法控制 - 已启用"
    } else {
        "输入法控制 - 已禁用"
    };

    info!(
        "更新托盘图标 - enabled: {}, icon_id: {}",
        enabled, icon_id
    );

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uFlags: NIF_ICON | NIF_TIP,
        hIcon: unsafe {
            let icon = LoadIconW(h_instance, PCWSTR(icon_id as *const u16))?;
            info!("加载图标句柄: {:?}", icon);
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


// 添加托盘图标
pub fn add_tray_icon(hwnd: HWND) -> windows::core::Result<()> {
    let h_instance = unsafe { GetModuleHandleW(None) }?;
    let config = CONFIG.read().unwrap();

    let icon_id = if config.master_switch {
        constants::IDI_ICON_ACTIVE
    } else {
        constants::IDI_ICON1
    };
    let tooltip = if config.master_switch {
        "输入法控制 - 已启用"
    } else {
        "输入法控制 - 已禁用"
    };

    info!("配置: {}", config.master_switch);
    info!("加载图标 ID: {}", icon_id);
    info!("添加托盘图标提示: {}", tooltip);

    let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        hWnd: hwnd,
        uCallbackMessage: constants::NOTIFYICONMESSAGE,
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
