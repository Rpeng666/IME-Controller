use log::{Level, LevelFilter, Metadata, Record};
use simplelog::*;
use std::fs::File;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let log_file = File::create("app.log")?;

    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        log_file,
    )])?;

    log::info!("Logger initialized");
    Ok(())
}
