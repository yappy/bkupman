use std::path::Path;

use anyhow::{bail, Context, Result};
use getopts::Options;

fn check_empty_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    for entry in path.as_ref().read_dir()? {
        let _ = entry?;
        bail!("Directory is not empty");
    }

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

    // TODO
    unimplemented!()
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
