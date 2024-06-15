use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use getopts::Options;
use strum::{EnumMessage, IntoEnumIterator, VariantNames};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;

use super::{Config, RepositoryFile};
use crate::commands::CryptType;
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
    const DESC: &str = "Split and encrypt the latest files in the repository.";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let types = CryptType::iter()
        .map(|t| t.get_serializations()[0])
        .collect::<Vec<_>>()
        .join(", ");
    let type_descs = CryptType::iter()
        .map(|t| {
            format!(
                "    {:<5}: {}",
                t.get_serializations()[0],
                t.get_message().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optflag("t", "type", &format!("Encryption type ({types})"));

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts));
        println!("Encryption Types:\n{type_descs}");
        return Ok(());
    }
    let _matches = opts.parse(args).context(USAGE_HINT)?;

    super::process_with_config_lock(basedir, process_crypt)?;

    Ok(())
}
