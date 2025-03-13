use serde::{Deserialize, Serialize};
use tracing_subscriber::{
    EnvFilter,
    fmt::{format::Writer, time::FormatTime, writer::MakeWriterExt},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    filter: String,
    rolling_file: Option<(String, String)>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
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

pub fn init_log_file(filter: &str, directory: &str, file_name_prefix: &str) {
    set_log(Some(LogConfig {
        filter: filter.to_owned(),
        rolling_file: Some((directory.to_owned(), file_name_prefix.to_owned())),
    }))
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
        stdout = Some(std::io::stdout.with_max_level(tracing::Level::TRACE));
    }

    // tracing 初始化
    if let Some(stdout) = stdout {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_max_level(tracing::Level::TRACE)
            .with_timer(LocalTimer)
            .with_thread_ids(true)
            .with_env_filter(filter)
            .with_writer(stdout)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_max_level(tracing::Level::TRACE)
            .with_timer(LocalTimer)
            .with_thread_ids(true)
            .with_ansi(false)
            .with_env_filter(filter)
            .with_writer(logfile.unwrap())
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    };
}
