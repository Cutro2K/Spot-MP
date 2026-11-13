use std::fs::File;

use simplelog::{Config, LevelFilter, WriteLogger};

use crate::services::data_storage::path_resolver::PathResolver;

pub struct LogService;

impl LogService {
    pub fn init() {
        let config_folder = PathResolver::resolve_config_folder();
        let log_path = config_folder.join("spot-mp.log");

        WriteLogger::init(
            LevelFilter::Info,
            Config::default(),
            File::create(log_path).expect("Couldn't create the log file in config folder"),
        )
        .unwrap();
    }
}
