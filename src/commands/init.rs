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

fn init_dir(dirpath: impl AsRef<Path>, force: bool) -> Result<()> {
    let config = Config::default();
    let tomlpath = dirpath.as_ref().join(super::CONFIG_FILE_NAME);

    println!("Create dir: {}", super::DIRNAME_INBOX);
    super::with_force(force, || {
        fs::create_dir(dirpath.as_ref().join(super::DIRNAME_INBOX))
            .with_context(|| format!("Failed to create dir: {}", super::DIRNAME_INBOX))
    })?;
    println!("Create dir: {}", super::DIRNAME_REPO);
    super::with_force(force, || {
        fs::create_dir(dirpath.as_ref().join(super::DIRNAME_REPO))
            .with_context(|| format!("Failed to create dir: {}", super::DIRNAME_REPO))
    })?;
    println!("Create dir: {}", super::DIRNAME_CRYPT);
    super::with_force(force, || {
        fs::create_dir(dirpath.as_ref().join(super::DIRNAME_CRYPT))
            .with_context(|| format!("Failed to create dir: {}", super::DIRNAME_CRYPT))
    })?;

    println!("Create and write: {}", tomlpath.to_string_lossy());
    let toml = toml::to_string(&config).unwrap();
    super::with_force(force, || {
        fs::write(&tomlpath, toml)
            .with_context(|| format!("Failed to write: {}", tomlpath.to_string_lossy()))
    })?;

    println!("OK");
    Ok(())
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optflag("f", "force", "Ignore errors");

    let matches = opts.parse(args).context(USAGE_HINT)?;
    if matches.opt_present("h") {
        super::print_help(cmd, &opts);
        return Ok(());
    }
    let force = matches.opt_present("f");

    super::with_force(force, || check_empty_dir(basedir))?;
    init_dir(basedir, force)?;

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
        init_dir(&tmpdir, false)?;
        init_dir(&tmpdir, true)?;

        Ok(())
    }
}
