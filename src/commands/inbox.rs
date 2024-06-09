use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, ensure, Context, Result};
use getopts::Options;
use md5::{Digest, Md5};
use tokio::io::AsyncReadExt;
use tokio::runtime::Runtime;

use super::{Config, DIRNAME_INBOX};

#[derive(Default)]
struct ProcessStat {
    processed: AtomicU32,
    error: AtomicU32,
}

async fn process_dir(stat: Arc<ProcessStat>, dirpath: PathBuf) {
    println!("Process {}", dirpath.to_string_lossy());

    // get directory iterator (sync)
    let iter = match dirpath.read_dir() {
        Ok(iter) => iter,
        Err(err) => {
            println!("{err}");
            stat.error.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    // for each entry (sync)
    let mut handles = Vec::new();
    for entry in iter {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                println!("{err}");
                stat.error.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let path = entry.path();
        if path.is_file() {
            // execute on a separated thread
            let h = tokio::spawn(process_file(path));
            handles.push(h);
        } else {
            println!("Not a regular file {}", path.to_string_lossy());
            stat.error.fetch_add(1, Ordering::Relaxed);
        }
    }
    for h in handles {
        // JoinError happens only if cancel or panic
        match h.await.expect("unexpected JoinError") {
            Ok(()) => {
                stat.processed.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                println!("{:#}", err);
                stat.error.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

fn str_to_md5(s: &str) -> Result<[u8; super::MD5LEN]> {
    ensure!(s.len() == super::MD5STRLEN);

    let mut hash = [0; super::MD5LEN];
    for (i, x) in hash.iter_mut().enumerate() {
        let b = u8::from_str_radix(&s[(i * 2)..=(i * 2 + 1)], 16)?;
        *x = b;
    }

    Ok(hash)
}

async fn process_file(filepath: PathBuf) -> Result<()> {
    const BUFSIZE: usize = 64 * 1024;

    // only UTF-8 path is valid
    filepath
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path: {}", filepath.to_string_lossy()))?;

    // skip "*.md5sum"
    if let Some(rawext) = filepath.extension() {
        let ext = rawext.to_str().unwrap();
        if ext == super::MD5EXT {
            return Ok(());
        }
    }

    println!("File: {}", filepath.to_string_lossy());

    let filename = filepath.file_name().unwrap().to_str().unwrap();
    let md5filename = format!("{}.{}", filename, super::MD5EXT);
    let md5path = filepath.with_file_name(md5filename);

    // read md5 from text
    let mut md5str = tokio::fs::read_to_string(&md5path)
        .await
        .with_context(|| format!("Cannot read {}", md5path.to_string_lossy()))?;
    md5str.truncate(super::MD5STRLEN);
    let md5 = str_to_md5(&md5str)
        .with_context(|| format!("Failed to convert to MD5 {}", md5path.to_string_lossy()))?;

    // read the file and calc md5
    let mut fin = tokio::fs::File::open(&filepath).await?;
    let mut buf = vec![0u8; BUFSIZE];
    let mut hasher = Md5::new();
    loop {
        let read_size = fin.read(&mut buf).await?;
        if read_size == 0 {
            break;
        }
        hasher.update(&buf[..read_size]);
    }
    let result = hasher.finalize();

    // verify md5
    ensure!(*result == md5, "MD5 unmatch");

    // TODO
    println!("OK: {}", filepath.to_string_lossy());
    Ok(())
}

fn process_inbox(dirpath: &Path, mut config: Config) -> Result<Option<Config>> {
    let inbox_path = dirpath.join(DIRNAME_INBOX);

    let stat: Arc<ProcessStat> = Arc::new(Default::default());
    let rt = Runtime::new()?;
    rt.block_on(process_dir(Arc::clone(&stat), inbox_path));
    drop(rt);

    println!("Processed: {}", stat.processed.load(Ordering::Relaxed));
    println!("Error    : {}", stat.error.load(Ordering::Relaxed));

    // update toml
    config.system.update();
    Ok(Some(config))
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    let matches = opts.parse(args).context(USAGE_HINT)?;
    if matches.opt_present("h") {
        super::print_help(cmd, &opts);
        return Ok(());
    }

    super::process_with_config_lock(basedir, process_inbox)?;

    Ok(())
}
