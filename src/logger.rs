use log::LevelFilter;
use log4rs;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 log4rs 配置
    let config = log4rs::config::Config::builder()
        .appender(
            log4rs::config::Appender::builder()
                .build(
                    "file",
                    Box::new(
                        log4rs::append::rolling_file::RollingFileAppender::builder()
                            .build("app.log", 
                                Box::new(
                                    log4rs::append::rolling_file::policy::compound::CompoundPolicy::new(
                                        Box::new(log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger::new(2 * 1024 * 1024)), // 2MB
                                        Box::new(log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller::builder().build("app.{}.log", 3)?)
                                    )
                                )
                            )?
                    )
                )
        )
        .appender(
            log4rs::config::Appender::builder()
                .build(
                    "console",
                    Box::new(log4rs::append::console::ConsoleAppender::builder().build())
                )
        )
        .logger(
            log4rs::config::Logger::builder()
                .appender("file")
                .appender("console")
                .additive(false)
                .build("app", LevelFilter::Info)
        )
        .build(
            log4rs::config::Root::builder()
                .appender("file")
                .appender("console")
                .build(LevelFilter::Info)
        )?;

    log4rs::init_config(config)?;

    log::info!("日志系统已初始化，使用自动轮转");
    Ok(())
}
