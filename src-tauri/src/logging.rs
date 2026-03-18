use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() -> WorkerGuard {
    let log_dir = get_log_directory();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "cm-launcher.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,reqwest=warn"));

    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized, log directory: {}", log_dir.display());
    let config = crate::config::get_config();
    tracing::info!(
        "{} version: {}",
        config.product_name,
        env!("CARGO_PKG_VERSION")
    );

    cleanup_old_logs(&log_dir, 7);

    guard
}

fn get_log_directory() -> std::path::PathBuf {
    let config = crate::config::get_config();
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(config.app_identifier)
        .join("logs");

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Warning: Failed to create log directory: {}", e);
    }

    log_dir
}

fn cleanup_old_logs(log_dir: &std::path::Path, keep_days: u64) {
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(keep_days * 24 * 60 * 60);

    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("Failed to read log directory for cleanup: {}", e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.starts_with("cm-launcher.log") => name,
            _ => continue,
        };

        if filename == "cm-launcher.log" {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if modified < cutoff {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("Failed to remove old log file {:?}: {}", path, e);
            } else {
                tracing::debug!("Removed old log file: {:?}", path);
            }
        }
    }
}
