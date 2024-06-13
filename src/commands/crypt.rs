use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, ensure, Context, Result};
use getopts::Options;
use md5::{Digest, Md5};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;

use super::{Config, RepositoryFile};
use crate::util;

/*
#[derive(Default)]
struct ProcessStat {
    /// (tag, filename)
    processed: Mutex<Vec<(String, RepositoryFile)>>,
    error: AtomicU32,
}
     */

async fn process_file(inbox_path: &Path, repo_path: &Path) {}

async fn process_flagment(file_path: &Path, repo_path: &Path) -> Result<()> {
    unimplemented!()
}

fn process_crypt(dirpath: &Path, mut config: Config) -> Result<Option<Config>> {
    let repo_path = dirpath.join(super::DIRNAME_REPO);

    let rt = Runtime::new()?;
    // rt.block_on();
    drop(rt);

    // update toml
    config.system.update();
    Ok(Some(config))
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const DESC: &str = "Encrypt the latest files in the repository.";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts));
        return Ok(());
    }
    let _matches = opts.parse(args).context(USAGE_HINT)?;

    super::process_with_config_lock(basedir, process_crypt)?;

    Ok(())
}
