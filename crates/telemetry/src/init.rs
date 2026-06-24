use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_telemetry() -> anyhow::Result<WorkerGuard> {
    let cfg = &*config::CONF;
    let is_file = cfg.workspace.log_dir.is_some();
    let level: Level = cfg.app.level.into();

    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let fmt_event = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_source_location(true)
        .compact();

    let (make_writer, guard) = if is_file {
        let directory = cfg.workspace.get_log_dir()?;
        let writer = tracing_appender::rolling::daily(directory, "hozz.log");
        tracing_appender::non_blocking(writer)
    } else {
        tracing_appender::non_blocking(std::io::stdout())
    };

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .event_format(fmt_event)
        .with_writer(make_writer)
        .with_ansi(!is_file)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!(level = ?&cfg.app.level, "Telemetry initialized");

    Ok(guard)
}
