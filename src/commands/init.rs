use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use getopts::Options;

use super::Config;

fn check_empty_dir(dirpath: impl AsRef<Path>) -> Result<()> {
    if let Some(entry) = (dirpath.as_ref().read_dir()?).next() {
        let _ = entry?;
        bail!("Directory is not empty");
    }

    Ok(())
}

fn init_dir(dirpath: impl AsRef<Path>) -> Result<()> {
    let config = Config::default();
    let tomlpath = dirpath.as_ref().join(super::CONFIG_FILE_NAME);

    let text = toml::to_string(&config).unwrap();
    fs::write(tomlpath, text)?;

    Ok(())
}

pub fn init(cmd: &str, args: &[String]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    let matches = opts.parse(args).context(USAGE_HINT)?;

    if matches.opt_present("h") {
        crate::print_help(cmd, &opts);
        return Ok(());
    }

    check_empty_dir(".")?;
    init_dir(".")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_init() -> Result<()> {
        let tmpdir = TempDir::new(".")?;
        std::env::set_current_dir(&tmpdir)?;
        check_empty_dir(&tmpdir)?;

        Ok(())
    }
}
