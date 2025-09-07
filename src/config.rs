use crate::tray::icon::update_tray_icon;
use crate::tray::notifications::show_balloon_tip;
use crate::utils::hot_key::register_all_hotkeys;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use windows::Win32::UI::TextServices::HKL;
use windows::core::PCSTR;
use windows::Win32::{
    Foundation::{GetLastError, HWND, LPARAM, WPARAM},
    UI::Input::KeyboardAndMouse::*,
    UI::Input::Ime::{self, *},
    UI::WindowsAndMessaging::{SendMessageW, WM_INPUTLANGCHANGEREQUEST},
};
use winreg::enums::*;
use winreg::RegKey;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ImeMode {
    ChineseOnly, // 仅中文模式
    EnglishOnly, // 仅英文模式
}

impl Default for ImeMode {
    fn default() -> Self {
        ImeMode::ChineseOnly
    }
}

// Global state
lazy_static! {
    pub static ref CONFIG: RwLock<Config> = RwLock::new(Config::load());

    static ref CONFIG_PATH: PathBuf = {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // 去掉 exe 文件名
        path.push("config.json");
        path
    };
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub autostart: bool,
    pub excluded_apps: Vec<String>,
    pub hotkey_toggle: Option<String>, // 切换总开关快捷键 (如: "Alt+M")
    pub hotkey_switch_mode: Option<String>, // 切换中英文模式快捷键 (如: "Alt+S")
    pub ime_mode: ImeMode,             // 输入法模式
    pub master_switch: bool,           // 总开关
    pub show_notifications: bool,      // 是否显示通知
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: false,
            excluded_apps: vec!["ime-controller.exe".to_string()],
            hotkey_toggle: Some("Alt+M".to_string()), // Alt+M (总开关)
            hotkey_switch_mode: Some("CAPSLOCK".to_string()), // CAPSLOCK (切换中英文模式)
            ime_mode: ImeMode::default(),
            master_switch: true,       // 默认总开关为开启
            show_notifications: false, // 默认不显示通知，避免打扰
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(config_path) {
            info!("加载配置文件: {:?}", content);
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            info!("配置文件不存在，使用默认配置");
            let config = Config::default();
            if let Err(e) = config.save() {
                error!("保存默认配置文件失败: {}", e);
            }
            config
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::get_config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(config_path, json)
    }

    fn get_config_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::current_exe().unwrap().parent().unwrap());
        info!("配置文件目录: {:?}", path);
        path.push("config.json");
        path
    }
}

/// 切换总开关
pub fn toggle_master_switch(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.master_switch = !config.master_switch;
    let new_state = config.master_switch;
    config.save().ok();
    drop(config);

    let _ = update_tray_icon(hwnd, new_state);

    if CONFIG.read().unwrap().show_notifications {
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

/// 切换自动启动
pub fn toggle_autostart(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.autostart = !config.autostart;

    if let Ok(exe_path) = std::env::current_exe() {
        let exe_path_str = exe_path.to_string_lossy();
        match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE,
        ) {
            Ok(run_key) => {
                if config.autostart {
                    let _ = run_key.set_value("IME Controller", &exe_path_str.as_ref());
                    show_balloon_tip(hwnd, "开机自启动", "已设置开机自动启动");
                } else {
                    let _ = run_key.delete_value("IME Controller");
                    show_balloon_tip(hwnd, "开机自启动", "已取消开机自启动");
                }
            }
            Err(_) => {
                show_balloon_tip(hwnd, "错误", "无法访问注册表Run键");
            }
        }
    }

    config.save().ok();
}

/// 切换通知显示
pub fn toggle_notifications(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    config.show_notifications = !config.show_notifications;
    config.save().ok();

    if config.show_notifications {
        show_balloon_tip(hwnd, "设置", "已开启通知显示");
    }
}

/// 切换输入法模式
pub fn switch_ime_mode(hwnd: HWND, mode: ImeMode) {
    let mut config = CONFIG.write().unwrap();
    config.ime_mode = mode.clone();
    config.save().ok();
    drop(config);

    apply_ime_setting_to_current_window(hwnd, mode.clone());

    show_balloon_tip(
        hwnd,
        "模式更改",
        match mode {
            ImeMode::ChineseOnly => "已切换到强制中文模式",
            ImeMode::EnglishOnly => "已切换到强制英文模式",
        },
    );
}

/// 应用输入法设置到当前窗口
pub fn apply_ime_setting_to_current_window(hwnd: HWND, config: ImeMode) {
    unsafe {
        let mut buf: [u8; 9] = [0u8; 9];
        if GetKeyboardLayoutNameA(&mut buf).as_bool() {
            let lang_id = String::from_utf8_lossy(&buf);
            let lang_id_str = lang_id.trim_end_matches('\0');

            info!("当前键盘布局: {}", lang_id_str);

            // 只处理简体中文输入法
            if lang_id_str == "00000804" {
                let target_hkl = match config {
                    ImeMode::ChineseOnly => HKL(0x0804 as isize), // 中文(简体)
                    ImeMode::EnglishOnly => HKL(0x0409 as isize), // 英文(美式)
                };

                // 1. 获取默认IME窗口
                let default_ime_wnd = ImmGetDefaultIMEWnd(hwnd);
                
                if default_ime_wnd.0 != 0 {
                    // 2. 直接发送切换消息给IME窗口
                    let _ = SendMessageW(
                        default_ime_wnd,
                        WM_INPUTLANGCHANGEREQUEST,
                        WPARAM(0),
                        LPARAM(target_hkl.0),
                    );

                    // 3. 获取IME上下文并设置状态
                    let himc = ImmGetContext(default_ime_wnd);
                    if himc.0 != 0 {
                        if let ImeMode::ChineseOnly = config {
                            let _ = ImmSetOpenStatus(himc, true);
                            // 设置转换模式为中文
                            let mut conversion_mode = IME_CONVERSION_MODE::default();
                            let mut sentence_mode = IME_SENTENCE_MODE::default();
                            if ImmGetConversionStatus(himc, Some(&mut conversion_mode), Some(&mut sentence_mode)).as_bool() {
                                let _ = ImmSetConversionStatus(
                                    himc,
                                    IME_CONVERSION_MODE(IME_CMODE_NATIVE.0 | IME_CMODE_FULLSHAPE.0 | IME_CMODE_CHINESE.0),
                                    sentence_mode,
                                );
                            }
                        } else {
                            let _ = ImmSetOpenStatus(himc, false);
                            // 设置转换模式为英文
                            let _ = ImmSetConversionStatus(
                                himc,
                                IME_CONVERSION_MODE(IME_CMODE_ALPHANUMERIC.0),
                                IME_SENTENCE_MODE::default(),
                            );
                        }
                        let _ = ImmReleaseContext(default_ime_wnd, himc);
                    }
                    
                    // 4. 强制刷新输入法状态
                    let himc = ImmGetContext(default_ime_wnd);
                    if himc.0 != 0 {
                        let _ = ImmNotifyIME(
                            himc,
                            NOTIFY_IME_ACTION(NI_COMPOSITIONSTR.0),
                            NOTIFY_IME_INDEX(CPS_CANCEL.0),
                            0,
                        );
                        let _ = ImmReleaseContext(default_ime_wnd, himc);
                    }
                }

                // 4. 使用 ActivateKeyboardLayout 作为备选方案
                let flags = ACTIVATE_KEYBOARD_LAYOUT_FLAGS(
                    KLF_ACTIVATE.0 | KLF_SETFORPROCESS.0 | KLF_REORDER.0 | KLF_REPLACELANG.0 | KLF_RESET.0,
                );

                let result = ActivateKeyboardLayout(target_hkl, flags);

                if result.is_err() {
                    let err_code = GetLastError().0;
                    error!(
                        "ActivateKeyboardLayout 切换输入法失败，错误码: {}",
                        err_code
                    );

                    // 尝试使用其他方式加载输入法
                    let layout_str = match config {
                        ImeMode::ChineseOnly => "00000804",
                        ImeMode::EnglishOnly => "00000409",
                    };

                    // 确保字符串以null结尾
                    let mut layout_bytes = layout_str.as_bytes().to_vec();
                    layout_bytes.push(0);

                    // 加载键盘布局
                    let loaded_hkl = LoadKeyboardLayoutA(PCSTR(layout_bytes.as_ptr()), KLF_ACTIVATE);

                    // 检查是否加载成功
                    match loaded_hkl {
                        Ok(hkl) => {
                            if hkl.0 == 0 {
                                error!(
                                    "LoadKeyboardLayout 也失败了，可能系统中没有安装对应的输入法"
                                );
                            } else {
                                info!("通过 LoadKeyboardLayout 成功加载输入法布局");
                                // 重试激活当前布局
                                let retry_result = ActivateKeyboardLayout(target_hkl, flags);
                                if retry_result.is_err() {
                                    error!("重试激活输入法失败");
                                } else {
                                    info!("通过 LoadKeyboardLayout 成功切换到 {:?}", config);
                                }
                            }
                        }
                        Err(_) => {
                            error!("LoadKeyboardLayout 调用失败");
                        }
                    }
                } else {
                    info!("ActivateKeyboardLayout 成功切换到 {:?}", config);
                }
            }
        }
    }
}

/// 打开配置目录
pub fn open_config_directory() {
    let config_dir = CONFIG_PATH.parent().unwrap();
    if let Err(e) = open::that(config_dir) {
        eprintln!("无法打开配置目录: {:?}", e);
    }
}

/// 重新加载配置并刷新热键
pub fn reload_config_and_hotkeys(hwnd: HWND) {
    let mut config = CONFIG.write().unwrap();
    *config = Config::load();
    drop(config);
    register_all_hotkeys(hwnd);
    info!("配置重新加载完成");
}
