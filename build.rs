use std::env;
use std::process::Command;
use std::path::Path;

fn main() {
    // 编译资源文件
    let rc_path = "assets/app.rc";
    let res_path = "assets/app.res";

    // 检查是否需要重新编译资源文件
    let should_compile = !Path::new(res_path).exists() || 
                        Path::new(rc_path).metadata().unwrap().modified().unwrap() > 
                        Path::new(res_path).metadata().map(|m| m.modified().unwrap()).unwrap_or(std::time::UNIX_EPOCH);

    if should_compile {
        // 尝试找到并使用 rc.exe
        let rc_result = if let Ok(vs_path) = env::var("VS2022INSTALLDIR") {
            println!("cargo:rerun-if-env-changed=VS2022INSTALLDIR");
            let rc_exe = format!(
                "{}\\VC\\Tools\\MSVC\\14.XX.XXXXX\\bin\\Hostx64\\x64\\rc.exe",
                vs_path
            );
            Command::new(&rc_exe)
                .args(&["/fo", res_path, rc_path])
                .status()
        } else {
            // 尝试直接使用 rc.exe（如果在 PATH 中）
            Command::new("rc")
                .args(&["/fo", res_path, rc_path])
                .status()
        };

        match rc_result {
            Ok(status) if status.success() => {
                println!("cargo:warning=Resource file compiled successfully");
            }
            _ => {
                println!("cargo:warning=Failed to compile resource file with rc.exe");
                println!("cargo:warning=You may need to run from a Visual Studio Developer Command Prompt");
                // 如果编译失败，创建一个空的资源文件占位符
                std::fs::write(res_path, b"").expect("Failed to create placeholder resource file");
            }
        }
    }

    // 链接资源文件（如果存在）
    if Path::new(res_path).exists() {
        println!("cargo:rustc-link-arg=assets/app.res");
    }

    // 当资源文件改变时重新编译
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app_active.ico");
}
