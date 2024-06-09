use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use getopts::Options;

use super::Config;

fn check_empty_dir(dirpath: impl AsRef<Path>) -> Result<()> {
    for entry in dirpath.as_ref().read_dir()? {
        let entry = entry?;
        // ignore hidden file/dir
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with('.') {
                continue;
            }
        }
        bail!("Directory is not empty");
    }

    Ok(())
}

fn init_dir(dirpath: impl AsRef<Path>) -> Result<()> {
    let config = Config::default();
    let tomlpath = dirpath.as_ref().join(super::CONFIG_FILE_NAME);

    fs::create_dir(dirpath.as_ref().join(super::DIRNAME_INBOX))?;
    fs::create_dir(dirpath.as_ref().join(super::DIRNAME_REPO))?;
    fs::create_dir(dirpath.as_ref().join(super::DIRNAME_CRYPT))?;

    let toml = toml::to_string(&config).unwrap();
    fs::write(tomlpath, toml)?;

    Ok(())
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

    check_empty_dir(basedir)?;
    init_dir(basedir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_init() -> Result<()> {
        let tmpdir = TempDir::new(".")?;
        check_empty_dir(&tmpdir)?;
        init_dir(&tmpdir)?;

        Ok(())
    }
}
