use std::collections::{BTreeMap, BTreeSet};
use std::io::prelude::*;
use std::sync::OnceLock;
use std::{fs::OpenOptions, path::Path};

use anyhow::{anyhow, ensure, Result};
use chrono::Local;
use fs2::FileExt;
use regex::{Match, Regex};
use serde::{Deserialize, Serialize};

pub mod inbox;
pub mod init;
pub mod test_file;

type CommandFunc = Box<dyn Fn(&Path, &str, &[String]) -> Result<()> + Send + Sync + 'static>;
type CommandMap = BTreeMap<&'static str, CommandFunc>;

#[cfg(debug_assertions)]
fn add_debug_commands(table: &mut CommandMap) {
    table.insert("test-file", Box::new(test_file::entry));
}

#[cfg(not(debug_assertions))]
fn add_debug_commands(_table: &mut CommandMap) {}

pub fn dispatch_table() -> &'static BTreeMap<&'static str, CommandFunc> {
    static TABLE: OnceLock<CommandMap> = OnceLock::new();

    TABLE.get_or_init(|| {
        let mut table: CommandMap = BTreeMap::new();
        table.insert("init", Box::new(init::entry));
        table.insert("inbox", Box::new(inbox::entry));
        add_debug_commands(&mut table);

        table
    })
}

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
    #[serde(default)]
    repository: Repository,
}

#[derive(Debug, Serialize, Deserialize)]
struct System {
    version: u32,
    updated: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Repository {
    entries: BTreeMap<String, BTreeSet<RepositoryFile>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct RepositoryFile {
    name: String,
    md5: String,
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

/// Do process with locking config file.
///
/// 1. Open and exclusive-lock dirpath/config.toml
/// 1. Call proc
/// 1. If proc returns Some, overwrite to dirpath/config.toml
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

fn with_force(force: bool, proc: impl FnOnce() -> Result<()>) -> Result<()> {
    let res = proc();
    if force {
        if let Err(err) = res {
            println!("{:#}", err);
        }
        Ok(())
    } else {
        Ok(res?)
    }
}

fn split_filename(name: &str) -> Result<(&str, &str, &str)> {
    fn slice<'a>(name: &'a str, m: Option<Match>) -> &'a str {
        if let Some(m) = m {
            &name[m.start()..m.start() + m.len()]
        } else {
            ""
        }
    }

    // *YYYYDDMM[hhmmss].*
    // (not-dot)+ (num){8,14} "." (any)*
    let re = Regex::new(r"^([^.]+)([0-9]{8,14})\.(.*)$").unwrap();
    let caps = re
        .captures(name)
        .ok_or_else(|| anyhow!("Invalid file name: {name}"))?;

    let s1 = slice(name, caps.get(1)).trim_end_matches(['-', '_']);
    ensure!(!s1.is_empty(), "Invalid file name: {name}");

    Ok((s1, slice(name, caps.get(2)), slice(name, caps.get(3))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_filename() -> Result<()> {
        let (a, b, c) = split_filename("hello-world-_-_-20240101.tar.bz2")?;
        assert_eq!(a, "hello-world");
        assert_eq!(b, "20240101");
        assert_eq!(c, "tar.bz2");

        let r = split_filename(".gitignore");
        assert!(r.is_err());

        let r = split_filename("----20240101.tar.bz2");
        assert!(r.is_err());

        Ok(())
    }
}
