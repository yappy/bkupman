use std::io::ErrorKind;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use bytes::{BufMut, BytesMut};
use getopts::Options;
use log::{debug, info};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

use super::{Config, RepositoryFile};
use crate::commands::{Aes128GcmArgon2Param, CryptInfo, CryptType};
use crate::cryptutil::AesKey;
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

async fn process_file_aes(
    src_file_path: &Path,
    dst_dir_path: &Path,
    dst_info_path: &Path,
    rf: RepositoryFile,
    fragment_size: NonZeroU64,
    key: AesKey,
    argon2: Aes128GcmArgon2Param,
) -> Result<()> {
    // source file
    let mut fin = tokio::fs::File::open(src_file_path).await?;

    let bufsize = fragment_size.get() as usize;
    let mut rawbuf = vec![0u8; bufsize];
    let mut total_size = 0u64;
    let mut idx = 0u64;
    loop {
        let rsize = util::read_fully(&mut fin, &mut rawbuf).await?;
        if rsize == 0 {
            break;
        }
        let rawbuf = &rawbuf[..rsize];
        total_size += rsize as u64;

        // encrypt
        // use saved key
        // nonce: 96 bit = 12 byte, must generate new one every time
        let (nonce, encbuf) = cryptutil::encrypt_aes256gcm(&key, rawbuf)?;

        // fragment file name
        let dst_path = dst_dir_path.join(&format!("{}.{:0>6}", rf.name, idx));
        let mut fout = tokio::fs::File::create(&dst_path).await?;
        debug!("To: {}", dst_path.display());

        // Argon2 salt:16, m:4, t:4, p:4
        // aes256-gcm nonce:12
        // ciphertext (+ tag:16)
        let mut header_buf = BytesMut::with_capacity(64);
        header_buf.put(&argon2.salt[..]);
        header_buf.put_u32_le(argon2.m_cost);
        header_buf.put_u32_le(argon2.t_cost);
        header_buf.put_u32_le(argon2.p_cost);
        header_buf.put(&nonce[..]);

        debug!(
            "plain: {}, header: {}, crypted: {}",
            rawbuf.len(),
            header_buf.len(),
            encbuf.len()
        );

        fout.write_all(&header_buf).await?;
        fout.write_all(&encbuf).await?;
        drop(fout);

        idx += 1;
    }

    // save crypt matadata
    let info = CryptInfo {
        // don't save the AES key
        crypt: CryptType::Aes128GcmArgon2 { key: None, argon2 },
        total_size,
        fragment_size,
    };
    tokio::fs::write(&dst_info_path, toml::to_string(&info)?).await?;

    let total_count: u64 = idx;
    info!(
        "Complete: {} ({} files, {} bytes)",
        dst_dir_path.display(),
        total_count,
        total_size
    );

    Ok(())
}

/// Return tag if succeeded
async fn process_file(param: Arc<TaskParam>, tag: String, rf: RepositoryFile) -> Result<String> {
    let src_file_path = param.repo_path.join(&tag).join(&rf.name);
    let dst_dir_path = param.crypt_path.join(&tag);
    let dst_info_path = dst_dir_path.join(super::CRYPT_INFO_NAME);

    info!("Clean: {}", dst_dir_path.display());
    let result = tokio::fs::remove_dir_all(&dst_dir_path).await;
    if let Err(ref err) = result {
        if err.kind() != ErrorKind::NotFound {
            result.with_context(|| format!("Rmdir failed: {}", dst_dir_path.display()))?
        }
    }
    tokio::fs::create_dir_all(&dst_dir_path)
        .await
        .with_context(|| format!("Mkdir failed: {}", dst_dir_path.display()))?;

    info!(
        "Process: {}, Src {}, Dst {}",
        tag,
        src_file_path.display(),
        dst_dir_path.display()
    );

    match &param.ctype {
        CryptType::PlainText => process_file_plain(param, tag.clone(), rf).await?,
        CryptType::Aes128GcmArgon2 { key, argon2 } => {
            let key = key.ok_or_else(|| anyhow!("Encryption key is empty"))?;
            process_file_aes(
                &src_file_path,
                &dst_dir_path,
                &dst_info_path,
                rf,
                param.fragment_size,
                key,
                argon2.clone(),
            )
            .await?
        }
    }

    Ok(tag)
}

async fn process_files(
    param: Arc<TaskParam>,
    files: &[(String, RepositoryFile)],
) -> (Result<()>, Vec<String>) {
    info!("{} files to be processed", files.len());
    let handles: Vec<_> = files
        .iter()
        .map(|(tag, rf)| {
            let param = Arc::clone(&param);
            let tag = tag.clone();
            let rf = rf.clone();
            // create a task
            tokio::spawn(async move { process_file(param, tag, rf).await })
        })
        .collect();

    let mut succeeded_tags = vec![];
    let mut failed = 0;
    for h in handles {
        // JoinError happens only if cancel or panic
        match h.await.unwrap() {
            Ok(tag) => {
                succeeded_tags.push(tag);
            }
            Err(err) => {
                println!("{:#}", err);
                failed += 1;
            }
        }
    }
    info!("Succeeded: {}", succeeded_tags.len());
    info!("Failed   : {}", failed);

    let res = if failed == 0 {
        Ok(())
    } else {
        Err(anyhow!("One or more errors occurred"))
    };

    (res, succeeded_tags)
}

fn process_crypt(
    dirpath: &Path,
    mut config: Config,
    fragment_size: NonZeroU64,
) -> (Option<Config>, Result<()>) {
    let repo_path = dirpath.join(super::DIRNAME_REPO);
    let crypt_path = dirpath.join(super::DIRNAME_CRYPT);

    let param = Arc::new(TaskParam {
        ctype: config.crypt.clone(),
        fragment_size,
        repo_path,
        crypt_path,
    });

    // filter to pick up (latest && no crypt data)
    let latest_files_wo_crypt: Vec<_> = config
        .repository
        .entries
        .iter()
        .filter_map(|(k, v)| {
            let latest = v.first();
            if let Some(rf) = latest {
                if !rf.0.crypt {
                    Some((k.to_string(), rf.0.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let rt = Runtime::new().unwrap();
    let (res, succeeded_tags) = rt.block_on(process_files(param, &latest_files_wo_crypt));
    drop(rt);

    // update toml
    for tag in succeeded_tags.iter() {
        let ents = config.repository.entries.get_mut(tag).unwrap();
        let mut rf = ents.pop_first().unwrap();
        rf.0.crypt = true;
        ents.insert(rf);
    }
    config.system.update();

    (Some(config), res)
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const FRAGMENT_MIN: u64 = 1024 * 1024;
    const FRAGMENT_DEFAULT: &str = "64m";

    const DESC: &str = "Split and encrypt the latest files in the repository.";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optopt("f", "flagment-size", "Split fragment size", "<SIZE>");

    if util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts, None));
        return Ok(());
    }
    let matches = opts.parse(args).context(USAGE_HINT)?;

    let fragment = matches.opt_str("f").unwrap_or(FRAGMENT_DEFAULT.to_string());
    let fragment = util::parse_size(&fragment)?;
    if fragment < FRAGMENT_MIN {
        bail!("fragment size must be < {FRAGMENT_MIN}");
    }
    let fragment = NonZeroU64::new(fragment).unwrap();

    super::process_with_config_lock_force_save(basedir, |basedir, config| {
        process_crypt(basedir, config, fragment)
    })?;

    Ok(())
}
