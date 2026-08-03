use env_logger::Env;

pub(crate) fn configure_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}
