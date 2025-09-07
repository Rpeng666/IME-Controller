use crate::config::CONFIG;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::*;

pub fn show_balloon_tip(hwnd: HWND, title: &str, msg: &str) {
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
    for (i, c) in msg.encode_utf16().take(255).enumerate() {
        nid.szInfo[i] = c;
    }

    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &mut nid);
    }
}

pub fn show_hotkey_config_info(hwnd: HWND) {
    let config = CONFIG.read().unwrap();        

    let toggle_key = config
        .hotkey_toggle
        .as_ref()
        .unwrap_or(&"未设置".to_string())
        .clone();

    let switch_mode_key = config
        .hotkey_switch_mode
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
