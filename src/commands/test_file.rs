use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use getopts::Options;
use tokio::runtime::Runtime;

use super::Config;
use crate::util;

async fn create_test_file(path: PathBuf, size: u64, random: bool) -> Result<()> {
    println!("Create {} size={size} random={random}", path.display());

    let file = tokio::fs::File::create(&path).await?;
    if random {
        todo!()
    } else {
        file.set_len(size).await?;
    }
    drop(file);

    Ok(())
}

fn process_test_file(
    dirpath: &Path,
    _: Config,
    size: u64,
    count: usize,
    random: bool,
) -> Result<Option<Config>> {
    let inbox_path = dirpath.join(super::DIRNAME_INBOX);

    {
        let rt = Runtime::new()?;
        rt.block_on(async move {
            let mut handles = Vec::new();
            for i in 0..count {
                let dt = Local::now().format("%Y%m%d%H%M%S").to_string();
                let name = format!("testfile-{i:0>5}_{}.bin", dt);
                let path = inbox_path.join(name);
                let h = tokio::spawn(create_test_file(path, size, random));
                handles.push(h);
            }
            for h in handles {
                // JoinError happens only if cancel or panic
                if let Err(err) = h.await.expect("unexpected JoinError") {
                    println!("{:#}", err);
                }
            }
        });
    }

    Ok(None)
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const DESC: &str = "Create test file(s).";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optopt("s", "size", "File size (default=1m)", "SIZE");
    opts.optopt("c", "count", "File count (default=1)", "COUNT");
    opts.optflag("r", "random", "Fill with random data (default=false)");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", util::create_help(cmd, DESC, &opts));
        return Ok(());
    }
    let matches = opts.parse(args).context(USAGE_HINT)?;
    let sizestr = matches.opt_str("s").unwrap_or("1m".into());
    let size = util::parse_size(&sizestr)?;
    let count = matches.opt_get_default("c", 1)?;
    let random = matches.opt_present("r");

    super::process_with_config_lock(basedir, |dirpath, config| {
        process_test_file(dirpath, config, size, count, random)
    })
}
