use common::logging::CommonLogger;


fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}
