use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use getopts::Options;
use tokio::runtime::Runtime;

use super::{Config, DIRNAME_INBOX};

#[derive(Default)]
struct ProcessStat {
    processed: AtomicU32,
    error: AtomicU32,
}

async fn process_dir(stat: Arc<ProcessStat>, dirpath: PathBuf) {
    println!("Process {}", dirpath.to_string_lossy());

    // get directory iterator
    let iter = match dirpath.read_dir() {
        Ok(iter) => iter,
        Err(err) => {
            println!("{err}");
            stat.error.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    // for each entry
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
            let h = tokio::spawn(process_file(Arc::clone(&stat), path));
            handles.push(h);
        } else {
            println!("Not a regular file {}", path.to_string_lossy());
            stat.error.fetch_add(1, Ordering::Relaxed);
        }
    }
    for h in handles {
        h.await.unwrap();
    }
}

async fn process_file(stat: Arc<ProcessStat>, filepath: PathBuf) {
    println!("Process start: {}", filepath.to_string_lossy());

    let sec = (stat.processed.load(Ordering::Relaxed) % 5) as u64;
    tokio::time::sleep(std::time::Duration::from_secs(sec)).await;

    println!("Process end: {}", filepath.to_string_lossy());

    // TODO
    stat.processed.fetch_add(1, Ordering::Relaxed);
}

fn process_inbox(dirpath: &Path, mut config: Config) -> Result<Option<Config>> {
    let inbox_path = dirpath.join(DIRNAME_INBOX);

    let stat: Arc<ProcessStat> = Arc::new(Default::default());
    let rt = Runtime::new()?;
    rt.block_on(process_dir(Arc::clone(&stat), inbox_path));

    println!("Processed: {}", stat.processed.load(Ordering::Relaxed));
    println!("Error    : {}", stat.error.load(Ordering::Relaxed));

    // update toml
    config.system.update();
    Ok(Some(config))
}

pub fn entry(cmd: &str, args: &[String]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    let matches = opts.parse(args).context(USAGE_HINT)?;
    if matches.opt_present("h") {
        super::print_help(cmd, &opts);
        return Ok(());
    }

    super::process_with_config_lock(".", process_inbox)?;

    Ok(())
}
