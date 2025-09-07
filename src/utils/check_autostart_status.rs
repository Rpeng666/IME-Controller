use crate::config::CONFIG;
use log::{error, info};
use winreg::enums::*;
use winreg::RegKey;

// 检查开机自启动状态
pub fn check_autostart_status() -> bool {
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
    {
        Ok(run_key) => match run_key.get_value::<String, _>("IME Controller") {
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
        },
        Err(e) => {
            error!("无法访问注册表Run键: {:?}", e);
            false
        }
    }
}

// 同步配置与注册表状态
pub fn sync_autostart_config() {
    let registry_autostart = check_autostart_status();
    let mut config = CONFIG.write().unwrap();

    if config.autostart != registry_autostart {
        info!(
            "同步开机自启动配置: 配置文件={}, 注册表={}",
            config.autostart, registry_autostart
        );
        config.autostart = registry_autostart;
        config.save().unwrap_or_else(|e| {
            error!("保存配置文件失败: {}", e);
        });
    }
}

// 检测是否可能是开机自启动
pub fn is_likely_autostart() -> bool {
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
