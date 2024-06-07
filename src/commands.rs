use serde::{Deserialize, Serialize};

pub mod init;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    created: String,
}
