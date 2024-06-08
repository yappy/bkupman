use std::path::Path;

use anyhow::{Context, Result};
use getopts::Options;

use super::{Config, DIRNAME_INBOX};

#[derive(Default, Clone, Copy)]
struct ProcessStat {
    processed: u32,
    error: u32,
}

fn process_file(mut stat: ProcessStat, filepath: &Path) -> ProcessStat {
    println!("Process {}", filepath.to_string_lossy());

    stat.processed += 1;
    stat
}

fn process_dir(mut stat: ProcessStat, dirpath: &Path) -> ProcessStat {
    println!("Process {}", dirpath.to_string_lossy());

    let iter = match dirpath.read_dir() {
        Ok(iter) => iter,
        Err(err) => {
            println!("{err}");
            stat.error += 1;
            return stat;
        }
    };

    for entry in iter {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                println!("{err}");
                stat.error += 1;
                continue;
            }
        };
        let path = entry.path();
        if path.is_symlink() {
            println!("Symbolic link is found. Ignored.");
            stat.error += 1;
        } else if path.is_dir() {
            stat = process_dir(stat, &path);
        } else if path.is_file() {
            stat = process_file(stat, &path);
        } else {
            println!("Unknown file type");
            stat.error += 1;
        }
    }

    stat
}

fn process_inbox(dirpath: &Path, mut config: Config) -> Result<Option<Config>> {
    let inbox_path = dirpath.join(DIRNAME_INBOX);

    let stat: ProcessStat = Default::default();
    let stat = process_dir(stat, &inbox_path);
    println!("Processed: {}", stat.processed);
    println!("Error    : {}", stat.error);

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
