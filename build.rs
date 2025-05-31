use std::env;
use std::process::Command;

fn main() {
    // 编译资源文件
    let rc_path = "assets/app.rc";
    let res_path = "assets/app.res";

    // 设置 Visual Studio 工具路径
    if let Ok(vs_path) = env::var("VS2022INSTALLDIR") {
        let rc_exe = format!(
            "{}\\VC\\Tools\\MSVC\\14.XX.XXXXX\\bin\\Hostx64\\x64\\rc.exe",
            vs_path
        );
        Command::new(&rc_exe)
            .args(&["/fo", res_path, rc_path])
            .status()
            .expect("Failed to compile resource file");
    } else {
        // 尝试直接使用 rc.exe（如果在 PATH 中）
        Command::new("rc")
            .args(&["/fo", res_path, rc_path])
            .status()
            .expect("Failed to compile resource file");
    }
    // 链接资源文件
    println!("cargo:rustc-link-arg=assets/app.res");

    // 当资源文件改变时重新编译
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");
}
