use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, ensure, Context, Result};
use getopts::Options;
use strum::{EnumMessage, IntoEnumIterator, VariantNames};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;

use super::{inbox, Config, RepositoryFile};
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

struct TaskParam {
    ctype: CryptType,
    repo_path: PathBuf,
    crypt_path: PathBuf,
}

async fn process_tag(param: Arc<TaskParam>, tag: String) {
    println!("Process: {tag}");
}

async fn process_tags(param: Arc<TaskParam>, tags: &[impl ToString]) {
    let handles: Vec<_> = tags
        .iter()
        .map(|tag| {
            let param = Arc::clone(&param);
            let tag = tag.to_string();
            tokio::spawn( process_tag(param, tag) )
        })
        .collect();

    for h in handles {
        let result = h.await;
    }
}

fn process_crypt(dirpath: &Path, mut config: Config, ctype: CryptType) -> Result<Option<Config>> {
    let repo_path = dirpath.join(super::DIRNAME_REPO);
    let crypt_path = dirpath.join(super::DIRNAME_CRYPT);

    let param = Arc::new(TaskParam {
        ctype,
        repo_path,
        crypt_path,
    });

    // filter to pick up (latest && crypt entry is empty)
    let latest_tags_wo_crypt: Vec<String> = config
        .repository
        .entries
        .iter()
        .filter_map(|(k, v)| {
            let latest = v.first();
            if let Some(rf) = latest {
                if rf.0.crypt.is_none() {
                    Some(k.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let rt = Runtime::new()?;
    rt.block_on(process_tags(param, &latest_tags_wo_crypt));
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
    opts.reqopt("t", "type", &format!("Encryption type ({types})"), "<TYPE>");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts));
        println!("Encryption Types:\n{type_descs}");
        return Ok(());
    }
    let matches = opts.parse(args).context(USAGE_HINT)?;

    let typestr = matches.opt_str("t").unwrap();
    let ctype = CryptType::from_str(&typestr).context("invalid crypt type")?;

    super::process_with_config_lock(basedir,  |basedir, config|{
        process_crypt(basedir, config, ctype)
})?;

    Ok(())
}
