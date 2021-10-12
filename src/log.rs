use std::env;
use chrono::Local;
use fern::{self, Dispatch};
use log::LevelFilter;

#[derive(Debug, Copy, Clone, Hash)]
pub enum Verbosity {
    Info,
    Debug,
    Trace,
}

impl Verbosity {
    pub fn init_rlc_log_system_with_verbosity(verbose: Verbosity) -> Result<(), fern::InitError> {
        let mut dispatch = Dispatch::new();

        dispatch = match verbose {
            Verbosity::Info => dispatch.level(LevelFilter::Info),
            Verbosity::Debug => dispatch.level(LevelFilter::Debug),
            Verbosity::Trace => dispatch.level(LevelFilter::Trace),
        }.level_for(
            "rlc",
            if cfg!(debug_assertion) {LevelFilter::Debug} else {LevelFilter::Info}
        );

        if let Some(log_file_path) = env::var_os("RLC_LOG_FILE_PATH") {
            let file_dispatch = Dispatch::new()
                .filter(|metadata| metadata.target() == "rlc-output")
                .format(|callback, args, record| {
                    callback.finish(format_args!(
                        "{} |RLC OUTPUT-{:5}| {}",
                        Local::now(),
                        record.level(),
                        args,
                    ))
                })
                .chain(fern::log_file(log_file_path)?);
            dispatch = dispatch.chain(file_dispatch);
        }

        let stdout_dispatch = Dispatch::new()
            .format(|callback, args,record| {
                callback.finish(format_args!(
                    "{} |{:5}| [{}] {}",
                    Local::now(),
                    record.level(),
                    record.target(),
                    args,
                ))
            })
            .chain(std::io::stdout());

        dispatch.chain(stdout_dispatch).apply()?;
        Ok(())
    }
}

#[macro_export]
macro_rules! rlc_info {
    ($($arg:tt)+) => (
        ::log::info!(target: "rlc-output", $($arg)+)
    );
}

#[macro_export]
macro_rules! rlc_error {
    ($($arg:tt)+) => (
        ::log::error!(target: "rlc-output", $($arg)+)
    );
}

pub fn rlc_error_and_exit(msg: impl AsRef<str>) -> ! {
    rlc_error!("Fatal error in RLC: {}", msg.as_ref());
    std::process::exit(1)
}