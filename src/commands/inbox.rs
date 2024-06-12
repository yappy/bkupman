use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, ensure, Context, Result};
use getopts::Options;
use md5::{Digest, Md5};
use tokio::io::AsyncReadExt;
use tokio::runtime::Runtime;

use super::{Config, RepositoryFile};

#[derive(Default)]
struct ProcessStat {
    /// (tag, filename)
    processed: Mutex<Vec<(String, RepositoryFile)>>,
    error: AtomicU32,
}

async fn process_dir(stat: Arc<ProcessStat>, inbox_path: &Path, repo_path: &Path) {
    println!("Process {}", inbox_path.to_string_lossy());

    // get directory iterator (sync)
    let iter = match inbox_path.read_dir() {
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
        let repo_path = PathBuf::from(repo_path);
        if path.is_file() {
            // execute on a separated thread
            let h = tokio::spawn(async move { process_file(&path, &repo_path).await });
            handles.push(h);
        } else {
            println!("Not a regular file {}", path.to_string_lossy());
            stat.error.fetch_add(1, Ordering::Relaxed);
        }
    }
    for h in handles {
        // JoinError happens only if cancel or panic
        match h.await.expect("unexpected JoinError") {
            Ok(result) => {
                let mut p = stat.processed.lock().unwrap();
                if let Some(tuple) = result {
                    p.push(tuple);
                }
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

async fn process_file(
    file_path: &Path,
    repo_path: &Path,
) -> Result<Option<(String, RepositoryFile)>> {
    const BUFSIZE: usize = 64 * 1024;

    // only UTF-8 path is valid
    file_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path: {}", file_path.to_string_lossy()))?;

    // skip "*.md5sum"
    if let Some(rawext) = file_path.extension() {
        let ext = rawext.to_str().unwrap();
        if ext == super::MD5EXT {
            return Ok(None);
        }
    }

    println!("File: {}", file_path.to_string_lossy());

    let filename = file_path.file_name().unwrap().to_str().unwrap();
    let (tag, date, ext) = super::split_filename(filename)?;
    let md5filename = format!("{}.{}", filename, super::MD5EXT);
    let md5path = file_path.with_file_name(md5filename);

    // read md5 from text
    let mut md5str = tokio::fs::read_to_string(&md5path)
        .await
        .with_context(|| format!("Cannot read {}", md5path.to_string_lossy()))?;
    md5str.truncate(super::MD5STRLEN);
    let md5 = str_to_md5(&md5str)
        .with_context(|| format!("Failed to convert to MD5 {}", md5path.to_string_lossy()))?;

    // read the file and calc md5
    let mut fin = tokio::fs::File::open(&file_path).await?;
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
    println!("MD5 verify OK: {}", file_path.to_string_lossy());

    // copy
    let destdir = repo_path.join(tag);
    tokio::fs::create_dir_all(&destdir).await?;
    let dest_file_name = format!("{tag}_{date}.{ext}");
    let destfile = destdir.join(&dest_file_name);
    let size = tokio::fs::copy(&file_path, &destfile).await?;
    println!(
        "Copy OK: {} => {} ({} B)",
        file_path.to_string_lossy(),
        destfile.to_string_lossy(),
        size
    );

    tokio::fs::remove_file(&file_path).await?;
    println!("Delete OK: {}", file_path.to_string_lossy());
    tokio::fs::remove_file(&md5path).await?;
    println!("Delete OK: {}", md5path.to_string_lossy());

    Ok(Some((
        tag.to_string(),
        RepositoryFile {
            name: dest_file_name,
            md5: md5str,
        },
    )))
}

fn process_inbox(dirpath: &Path, mut config: Config) -> Result<Option<Config>> {
    let inbox_path = dirpath.join(super::DIRNAME_INBOX);
    let repo_path = dirpath.join(super::DIRNAME_REPO);

    let stat: Arc<ProcessStat> = Arc::new(Default::default());
    let rt = Runtime::new()?;
    rt.block_on(process_dir(Arc::clone(&stat), &inbox_path, &repo_path));
    drop(rt);

    let processed = stat.processed.lock().unwrap();
    println!("Processed: {}", processed.len());
    println!("Error    : {}", stat.error.load(Ordering::Relaxed));

    // update toml
    for (tag, rf) in processed.iter() {
        match config.repository.entries.get_mut(tag) {
            Some(set) => {
                // insert to the set
                set.insert(rf.clone());
            }
            None => {
                // create a new set and insert to it
                // insert to the map
                let mut set = BTreeSet::new();
                set.insert(rf.clone());
                config.repository.entries.insert(tag.clone(), set);
            }
        }
    }
    config.system.update();
    Ok(Some(config))
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const DESC: &str = "Create test file(s).";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", crate::util::create_help(cmd, DESC, &opts));
        return Ok(());
    }
    let _matches = opts.parse(args).context(USAGE_HINT)?;

    super::process_with_config_lock(basedir, process_inbox)?;

    Ok(())
}
