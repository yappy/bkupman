use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use getopts::Options;
use md5::{Digest, Md5};
use tokio::{io::AsyncWriteExt, runtime::Runtime};

use super::Config;
use crate::util;

/// Notice: random fill is very slow on debug build.
async fn create_test_file(path: PathBuf, md5path: PathBuf, size: u64, random: bool) -> Result<()> {
    println!("Create {} size={size} random={random}", path.display());

    let mut hasher = Md5::new();
    {
        let mut file = tokio::fs::File::create(&path).await?;
        let mut rest = size as usize;
        let mut buf = vec![0; 64 * 1024];
        if random {
            let mut state = util::seed64();
            while rest > 0 {
                state = util::xorshift64_fill(&mut buf, state);
                let wsize = rest.min(buf.len());
                file.write_all(&buf[0..wsize]).await?;
                rest -= wsize;
                hasher.update(&buf[0..wsize]);
            }
        } else {
            file.set_len(size).await?;
            while rest > 0 {
                let wsize = rest.min(buf.len());
                rest -= wsize;
                hasher.update(&buf[0..wsize]);
            }
        }
    }
    let md5 = hasher.finalize();
    {
        let md5str = util::md5_to_str(&md5);
        let mut file = tokio::fs::File::create(&md5path).await?;
        file.write_all(md5str.as_bytes()).await?;
    }

    println!("OK: {}", path.display());
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
                let md5name = format!("{name}.{}", super::MD5EXT);
                let path = inbox_path.join(name);
                let md5path = inbox_path.join(md5name);
                let h = tokio::spawn(create_test_file(path, md5path, size, random));
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

    if util::find_option(&args, &["-h", "--help"]) {
        println!("{}", util::create_help(cmd, DESC, &opts, None));
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
