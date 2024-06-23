use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use getopts::Options;
use log::{debug, info};
use strum::{EnumMessage, IntoEnumIterator};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

use super::{Config, RepositoryFile};
use crate::commands::CryptType;
use crate::{cryptutil, util};

/*
#[derive(Default)]
struct ProcessStat {
    /// (tag, filename)
    processed: Mutex<Vec<(String, RepositoryFile)>>,
    error: AtomicU32,
}*/

struct TaskParam {
    ctype: CryptType,
    fragment_size: NonZeroU64,
    repo_path: PathBuf,
    crypt_path: PathBuf,
}

async fn process_file_plain(
    _param: Arc<TaskParam>,
    _tag: String,
    _rf: RepositoryFile,
) -> Result<()> {
    Ok(())
}

async fn process_file_aes(param: Arc<TaskParam>, tag: String, rf: RepositoryFile) -> Result<()> {
    let src_path = param.repo_path.join(&tag).join(&rf.name);
    let dst_dir_path = param.crypt_path.join(&tag);
    info!(
        "Process: {tag}, Src {}, Dst {}",
        src_path.display(),
        dst_dir_path.display()
    );

    tokio::fs::create_dir_all(&dst_dir_path).await?;

    // source file
    let mut fin = tokio::fs::File::open(src_path).await?;

    // keylen = 256 bit = 32 byte
    // TODO: change to true key
    let key = cryptutil::AesKey::default();

    let bufsize = param.fragment_size.get() as usize;
    let mut rawbuf = vec![0u8; bufsize];
    let mut idx = 0u64;
    loop {
        let dst_path = dst_dir_path.join(&format!("{}.{:0>6}", rf.name, idx));

        let rsize = util::read_fully(&mut fin, &mut rawbuf).await?;
        if rsize == 0 {
            break;
        }

        let rawbuf = &rawbuf[..rsize];

        // 96 bit = 12 byte, must generate new one every time
        let (nonce, encbuf) = cryptutil::encrypt_aes256gcm(&key, rawbuf)?;

        debug!(
            "plain: {}, nonce: {}, crypted: {}",
            rawbuf.len(),
            nonce.len(),
            encbuf.len()
        );
        info!("To: {}", dst_path.display());

        let mut fout = tokio::fs::File::create(&dst_path).await?;
        fout.write_all(&nonce).await?;
        fout.write_all(&encbuf).await?;

        idx += 1;
    }

    Ok(())
}

async fn process_files(param: Arc<TaskParam>, files: &[(String, RepositoryFile)]) {
    let handles: Vec<_> = files
        .iter()
        .map(|tuple| {
            let param = Arc::clone(&param);
            let tag = tuple.0.to_string();
            let rf = tuple.1.clone();
            // create a task
            tokio::spawn(async move {
                match param.ctype {
                    CryptType::PlainText => process_file_plain(param, tag, rf).await,
                    CryptType::Aes128Gcm => process_file_aes(param, tag, rf).await,
                }
            })
        })
        .collect();

    for h in handles {
        // JoinError happens only if cancel or panic
        match h.await.unwrap() {
            Ok(()) => {}
            Err(err) => {
                println!("{:#}", err);
            }
        }
    }
}

fn process_crypt(
    dirpath: &Path,
    mut config: Config,
    ctype: CryptType,
    fragment_size: NonZeroU64,
) -> Result<Option<Config>> {
    let repo_path = dirpath.join(super::DIRNAME_REPO);
    let crypt_path = dirpath.join(super::DIRNAME_CRYPT);

    let param = Arc::new(TaskParam {
        ctype,
        fragment_size,
        repo_path,
        crypt_path,
    });

    // filter to pick up (latest && crypt entry is empty)
    let latest_files_wo_crypt: Vec<_> = config
        .repository
        .entries
        .iter()
        .filter_map(|(k, v)| {
            let latest = v.first();
            if let Some(rf) = latest {
                if rf.0.crypt.is_none() {
                    Some((k.to_string(), rf.0.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let rt = Runtime::new()?;
    rt.block_on(process_files(param, &latest_files_wo_crypt));
    drop(rt);

    // update toml
    config.system.update();
    Ok(Some(config))
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const FRAGMENT_MIN: u64 = 1024 * 1024;
    const FRAGMENT_DEFAULT: &str = "64m";

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
    opts.optopt("f", "flagment-size", "Split fragment size", "<SIZE>");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts));
        println!("Encryption Types:\n{type_descs}");
        return Ok(());
    }
    let matches = opts.parse(args).context(USAGE_HINT)?;

    let typestr = matches.opt_str("t").unwrap();
    let ctype = CryptType::from_str(&typestr).context("invalid crypt type")?;
    let fragment = matches.opt_str("f").unwrap_or(FRAGMENT_DEFAULT.to_string());
    let fragment = util::parse_size(&fragment)?;
    if fragment < FRAGMENT_MIN {
        bail!("fragment size must be < {FRAGMENT_MIN}");
    }
    let fragment = NonZeroU64::new(fragment).unwrap();

    super::process_with_config_lock(basedir, |basedir, config| {
        process_crypt(basedir, config, ctype, fragment)
    })?;

    Ok(())
}
