use core::fmt;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::io::prelude::*;
use std::num::NonZeroU64;
use std::str::FromStr;
use std::{fs::OpenOptions, path::Path};

use anyhow::{anyhow, ensure, Context, Result};
use chrono::Local;
use fs2::FileExt;
use log::warn;
use regex::{Match, Regex};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumMessage, EnumString, IntoEnumIterator};

use crate::cryptutil;

pub mod crypt;
pub mod inbox;
pub mod init;
pub mod key;
pub mod test_file;

const CONFIG_FILE_NAME: &str = "config.toml";
const DIRNAME_INBOX: &str = "inbox";
const DIRNAME_REPO: &str = "repo";
const DIRNAME_CRYPT: &str = "crypt";

const MD5EXT: &str = "md5sum";

#[derive(EnumString, EnumMessage, EnumIter)]
enum CommandType {
    #[strum(serialize = "init", message = "Initialize directory as repository")]
    Init,
    #[strum(serialize = "key", message = "Set encrypt/decrypt key")]
    Key,
    #[strum(serialize = "inbox", message = "Process new files in inbox/")]
    Inbox,
    #[strum(serialize = "crypt", message = "Split and encrypt files in repo/")]
    Crypt,

    #[strum(serialize = "test-file", message = "Create test file(s) into inbox/")]
    TestFile,
}

pub fn dispatch_subcommand(basedir: impl AsRef<Path>, argv: &[String]) -> Result<()> {
    assert!(!argv.is_empty());

    let basedir = basedir.as_ref();
    let cmd = &argv[0];
    let args = &argv[1..];

    let ctype = CommandType::from_str(cmd).context("Subcommand not found")?;

    match ctype {
        CommandType::Init => init::entry(basedir, cmd, args),
        CommandType::Key => key::entry(basedir, cmd, args),
        CommandType::Inbox => inbox::entry(basedir, cmd, args),
        CommandType::Crypt => crypt::entry(basedir, cmd, args),
        CommandType::TestFile => test_file::entry(basedir, cmd, args),
    }
}

pub fn subcommands_help() -> String {
    let mut help = String::new();
    for ctype in CommandType::iter() {
        help += &format!(
            "{}\n    {}\n",
            ctype.get_serializations()[0],
            ctype.get_message().unwrap()
        );
    }

    help
}

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    system: System,
    #[serde(default)]
    crypt: CryptType,
    #[serde(default)]
    repository: Repository,
}

#[derive(Debug, Serialize, Deserialize)]
struct System {
    version: u32,
    updated: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum CryptType {
    #[default]
    PlainText,
    Aes128GcmArgon2 {
        key: Option<cryptutil::AesKey>,
        salt: cryptutil::Argon2Salt,
        m_cost: u32,
        t_cost: u32,
        p_cost: u32,
    },
}

impl fmt::Display for CryptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlainText => {
                write!(f, "PlainText (no encryption)")?;
            }
            Self::Aes128GcmArgon2 {
                key,
                salt,
                m_cost,
                t_cost,
                p_cost,
            } => {
                let salt_str = salt
                    .iter()
                    .fold(String::new(), |cur, b| cur + &format!("{:02x}", b));
                writeln!(f, "AES key derived from passphrase by Argon2")?;
                writeln!(f, "salt  : {salt_str}")?;
                writeln!(f, "m_cost: {m_cost}")?;
                writeln!(f, "t_cost: {t_cost}")?;
                writeln!(f, "p_cost: {p_cost}")?;
                if key.is_some() {
                    write!(f, "key   : SAVED (able to check passphrase)")?;
                } else {
                    write!(f, "key   : NODATA (passphrase needed)")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Repository {
    /// key = dirname, value = [RepositoryFile]
    entries: BTreeMap<String, BTreeSet<Reverse<RepositoryFile>>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct RepositoryFile {
    name: String,
    md5name: String,
    crypt: Option<CryptInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct CryptInfo {
    crypt: CryptType,
    /// fragment count = [Self::total_size] + [Self::fragment_size]
    total_size: u64,
    fragment_size: NonZeroU64,
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
            warn!("{:#}", err);
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
    let re = Regex::new(r"^([^.]*[^.0-9])([0-9]+)\.(.*)$").unwrap();
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

        let (a, b, c) = split_filename("testfile-00000_20240613165945.bin")?;
        assert_eq!(a, "testfile-00000");
        assert_eq!(b, "20240613165945");
        assert_eq!(c, "bin");

        let r = split_filename(".gitignore");
        assert!(r.is_err());

        let r = split_filename("----20240101.tar.bz2");
        assert!(r.is_err());

        Ok(())
    }
}
