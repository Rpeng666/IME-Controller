use log::{Level, LevelFilter};
use simplelog::*;
use std::fs::File;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    // 创建日志文件
    let log_file = File::create("app.log")?;

    // 初始化日志系统，同时写文件和控制台
    CombinedLogger::init(vec![
        // 写入文件
        WriteLogger::new(
            LevelFilter::Info, // 文件日志 Info 级别及以上
            ConfigBuilder::new().build(),
            log_file,
        ),
        // 输出到控制台
        TermLogger::new(
            LevelFilter::Info, // 控制台日志 Info 级别及以上
            ConfigBuilder::new().build(),
            TerminalMode::Mixed, // Mixed 模式：Info 打印到 stdout，Warn/Error 打印到 stderr
            ColorChoice::Auto,   // 自动使用颜色
        ),
    ])?;

    log::info!("日志系统已初始化，日志级别: {:?}", Level::Info);
    Ok(())
}
