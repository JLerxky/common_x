use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::Level;
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime, writer::MakeWriterExt},
    EnvFilter,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    max_level: String,
    filter: String,
    rolling_file: Option<(String, String)>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_level: "info".to_owned(),
            filter: "info".to_owned(),
            rolling_file: Default::default(),
        }
    }
}

pub fn init_log_filter(filter: &str) {
    set_log(Some(LogConfig {
        filter: filter.to_owned(),
        ..Default::default()
    }));
}

pub fn init_log(config: LogConfig) {
    set_log(Some(config));
}

fn set_log(log_config: Option<LogConfig>) {
    let log_config = log_config.unwrap_or_default();
    struct LocalTimer;
    impl FormatTime for LocalTimer {
        fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
            write!(w, "{}", chrono::Local::now().format("%m-%d %T%.3f"))
        }
    }
    let filter = EnvFilter::new(&log_config.filter);

    let mut logfile = None;
    let mut stdout = None;
    if let Some((directory, file_name_prefix)) = &log_config.rolling_file {
        // logfile
        logfile = Some(tracing_appender::rolling::daily(
            directory,
            file_name_prefix,
        ));
    } else {
        // stdout
        stdout = Some(
            std::io::stdout
                .with_max_level(tracing::Level::from_str(&log_config.max_level).unwrap()),
        );
    }

    // tracing 初始化
    if let Some(stdout) = stdout {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_timer(LocalTimer)
            .with_thread_ids(true)
            .with_env_filter(filter)
            .with_writer(stdout)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_timer(LocalTimer)
            .with_thread_ids(true)
            .with_env_filter(filter)
            .with_writer(logfile.unwrap())
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    };
}
