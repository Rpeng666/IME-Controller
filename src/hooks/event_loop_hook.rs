use crate::config::{Config, ImeMode, CONFIG};
use log::{debug, error, info, warn};
use windows::core::PCSTR;
use windows::Win32::UI::Input::KeyboardAndMouse::{ActivateKeyboardLayout, LoadKeyboardLayoutA};
use windows::Win32::UI::TextServices::HKL;
use windows::Win32::{
    Foundation::{GetLastError, HWND},
    UI::{Accessibility::HWINEVENTHOOK, Input::KeyboardAndMouse::*},
};
/// windows事件触发回调函数
pub unsafe extern "system" fn event_hook_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let config = CONFIG.read().unwrap().clone();

    if !config.master_switch {
        return;
    }

    let mut buf: [u8; 9] = [0u8; 9];
    if GetKeyboardLayoutNameA(&mut buf).as_bool() {
        let lang_id = String::from_utf8_lossy(&buf);
        let lang_id_str = lang_id.trim_end_matches('\0');

        info!("当前键盘布局: {}", lang_id_str);

        // 只处理简体中文输入法
        if lang_id_str == "00000804" {
            let target_hkl = match config.ime_mode {
                ImeMode::ChineseOnly => HKL(0x0804 as isize), // 中文(简体)
                ImeMode::EnglishOnly => HKL(0x0409 as isize), // 英文(美式)
            };

            // 使用多个标志组合来确保切换成功
            let flags = ACTIVATE_KEYBOARD_LAYOUT_FLAGS(
                KLF_ACTIVATE.0 | KLF_SETFORPROCESS.0 | KLF_REORDER.0,
            );

            let result = ActivateKeyboardLayout(target_hkl, flags);

            if result.is_err() {
                let err_code = GetLastError().0;
                error!(
                    "ActivateKeyboardLayout 切换输入法失败，错误码: {}",
                    err_code
                );

                // 尝试使用其他方式加载输入法
                let layout_str = match config.ime_mode {
                    ImeMode::ChineseOnly => "00000804",
                    ImeMode::EnglishOnly => "00000409",
                };

                // 确保字符串以null结尾
                let mut layout_bytes = layout_str.as_bytes().to_vec();
                layout_bytes.push(0);

                // 加载键盘布局
                let loaded_hkl =
                    unsafe { LoadKeyboardLayoutA(PCSTR(layout_bytes.as_ptr()), KLF_ACTIVATE) };

                // 检查是否加载成功
                match loaded_hkl {
                    Ok(hkl) => {
                        if hkl.0 == 0 {
                            error!("LoadKeyboardLayout 也失败了，可能系统中没有安装对应的输入法");
                        } else {
                            info!("通过 LoadKeyboardLayout 成功加载输入法布局");
                            // 重试激活当前布局
                            let retry_result = ActivateKeyboardLayout(target_hkl, flags);
                            if retry_result.is_err() {
                                error!("重试激活输入法失败");
                            } else {
                                info!("通过 LoadKeyboardLayout 成功切换到 {:?}", config.ime_mode);
                            }
                        }
                    }
                    Err(_) => {
                        error!("LoadKeyboardLayout 调用失败");
                    }
                }
            } else {
                info!("ActivateKeyboardLayout 成功切换到 {:?}", config.ime_mode);
            }
        }
    }
}
