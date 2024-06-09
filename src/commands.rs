use std::io::prelude::*;
use std::{fs::OpenOptions, path::Path};

use anyhow::Result;
use chrono::Local;
use fs2::FileExt;
use getopts::Options;
use serde::{Deserialize, Serialize};

pub mod inbox;
pub mod init;

const CONFIG_FILE_NAME: &str = "config.toml";
const DIRNAME_INBOX: &str = "inbox";
const DIRNAME_REPO: &str = "repo";
const DIRNAME_CRYPT: &str = "crypt";

const MD5EXT: &str = "md5sum";
// 128 bit
const MD5LEN: usize = 16;
const MD5STRLEN: usize = 32;

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    system: System,
}

#[derive(Debug, Serialize, Deserialize)]
struct System {
    version: u32,
    updated: String,
}

impl Default for System {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            updated: Local::now().to_string(),
        }
    }
}

impl System {
    fn update(&mut self) {
        self.updated = Local::now().to_string();
    }
}

fn process_with_config_lock(
    dirpath: impl AsRef<Path>,
    proc: impl FnOnce(&Path, Config) -> Result<Option<Config>>,
) -> Result<()> {
    let tomlpath = dirpath.as_ref().join(CONFIG_FILE_NAME);
    {
        // open with R/W
        let mut file = OpenOptions::new().read(true).write(true).open(tomlpath)?;
        file.try_lock_exclusive()?;
        let mut toml = String::new();
        file.read_to_string(&mut toml)?;
        let config: Config = toml::from_str(&toml)?;

        // if config is returned, overwrite (still locked)
        if let Some(config) = proc(dirpath.as_ref(), config)? {
            let toml = toml::to_string(&config).unwrap();
            file.seek(std::io::SeekFrom::Start(0))?;
            file.set_len(0)?;
            file.write_all(toml.as_bytes())?;
        }

        // unlock and close
    }
    Ok(())
}

fn print_help(program: &str, opts: &Options) {
    let brief = format!("Usage: {program} [options]");
    print!("{}", opts.usage(&brief));
}
