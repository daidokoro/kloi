use colored::Colorize;
use env_logger::Builder;
use log;
use std::io::Write;

pub fn init() {
    Builder::new()
        .filter(None, log::LevelFilter::Info)
        .parse_env("KLOI_LOG")
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Error => "error".red(),
                log::Level::Warn => "warn".yellow(),
                log::Level::Info => "info".green(),
                log::Level::Debug => "debug".magenta(),
                log::Level::Trace => "trace".blue(),
            };

            writeln!(buf, "[{}] {}", level, record.args())
        })
        .init();
}
