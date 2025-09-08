use crate::config::{Config, ImeMode, CONFIG};
use log::{debug, error, info, warn};
use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{Accessibility::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub unsafe extern "system" fn event_hook_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let master_switch = CONFIG.read().unwrap().master_switch;
    if !master_switch {
        return;
    }

    // Clone the config data we need
    let ime_mode = CONFIG.read().unwrap().ime_mode.clone();

    // 在新线程中执行，避免阻塞事件钩子
    let hwnd_raw = hwnd.0 as usize;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100)); // 等待窗口稳定
        unsafe {
            enforce_global_ime_mode(HWND(hwnd_raw as *mut std::ffi::c_void), &ime_mode);
        }
    });
}

pub unsafe fn enforce_global_ime_mode(hwnd: HWND, ime_mode: &ImeMode) {
    // 获取目标键盘布局
    let target_hkl = match ime_mode {
        ImeMode::ChineseOnly => HKL(0x0804 as *mut std::ffi::c_void), // 简体中文
        ImeMode::EnglishOnly => HKL(0x0409 as *mut std::ffi::c_void), // 英文
    };

    // 方法1: 激活目标键盘布局（全局）
    let result = ActivateKeyboardLayout(
        target_hkl,
        ACTIVATE_KEYBOARD_LAYOUT_FLAGS(KLF_ACTIVATE.0 | KLF_SETFORPROCESS.0 | KLF_REORDER.0),
    );

    match result {
        Ok(_) => {
            info!("ActivateKeyboardLayout 成功");

            // 发送消息确保UI更新
            let _ = PostMessageW(
                Some(HWND_BROADCAST),
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(target_hkl.0 as isize),
            );
        }
        Err(_) => {
            info!("ActivateKeyboardLayout 失败，尝试备用方法");

            // 方法2: 加载并激活键盘布局
            let layout_name = match ime_mode {
                ImeMode::ChineseOnly => "00000804",
                ImeMode::EnglishOnly => "00000409",
            };

            let wide_layout: Vec<u16> = layout_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            if let Ok(loaded_hkl) = LoadKeyboardLayoutW(PCWSTR(wide_layout.as_ptr()), KLF_ACTIVATE)
            {
                if !loaded_hkl.0.is_null() {
                    // 再次尝试激活
                    if ActivateKeyboardLayout(loaded_hkl, KLF_ACTIVATE).is_ok() {
                        info!("通过 LoadKeyboardLayout 激活成功");

                        // 发送全局消息确保UI更新
                        let _ = PostMessageW(
                            Some(HWND_BROADCAST),
                            WM_INPUTLANGCHANGEREQUEST,
                            WPARAM(0),
                            LPARAM(loaded_hkl.0 as isize),
                        );
                    } else {
                        error!("LoadKeyboardLayout 后激活失败");
                    }
                }
            } else {
                error!("LoadKeyboardLayout 失败");
            }
        }
    }

    // 方法3: 发送设置变更消息（总是执行）
    let _ = PostMessageW(Some(HWND_BROADCAST), WM_SETTINGCHANGE, WPARAM(0), LPARAM(0));

    info!("全局输入法强制设置完成: {:?}", ime_mode);

    // 验证当前布局
    verify_current_layout(ime_mode);
}

unsafe fn verify_current_layout(ime_mode: &ImeMode) {
    // 获取当前线程的键盘布局
    let current_hkl = GetKeyboardLayout(0); // 0表示当前线程
    let target_hkl = match ime_mode {
        ImeMode::ChineseOnly => HKL(0x0804 as *mut std::ffi::c_void),
        ImeMode::EnglishOnly => HKL(0x0409 as *mut std::ffi::c_void),
    };

    info!(
        "验证 - 当前HKL: 0x{:x}, 目标HKL: 0x{:x}",
        current_hkl.0 as usize, target_hkl.0 as usize
    );

    // 获取键盘布局名称进行验证
    let mut buf = [0u8; 9];
    if GetKeyboardLayoutNameA(&mut buf).is_ok() {
        let lang_id = String::from_utf8_lossy(&buf);
        let lang_id_str = lang_id.trim_end_matches('\0');
        info!("当前键盘布局名称: {}", lang_id_str);
    }
}
