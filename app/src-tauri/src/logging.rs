use std::{error::Error, path::Path};

pub(super) fn init(directory: &Path, verbose: bool) -> Result<(), Box<dyn Error>> {
    let rolling_file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("ice-audio-manager")
        .filename_suffix("log")
        .max_log_files(7)
        .build(directory)?;
    let level = if verbose { tracing::Level::DEBUG } else { tracing::Level::INFO };
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(level)
        .with_writer(rolling_file)
        .try_init()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}
