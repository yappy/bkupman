use anyhow::{Context, Result};
use getopts::Options;

use super::Config;

fn process_inbox(mut config: Config) -> Result<Option<Config>> {
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
        crate::print_help(cmd, &opts);
        return Ok(());
    }

    super::process_with_config_lock(".", process_inbox)?;

    Ok(())
}
