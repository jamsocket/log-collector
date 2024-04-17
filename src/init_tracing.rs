use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

pub fn init_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(layer)
        .with(filter)
        .init();
}
