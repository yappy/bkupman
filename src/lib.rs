use anyhow::{bail, ensure, Context, Result};
use getopts::Options;

fn print_help(program: &str, opts: Options) {
    let brief = format!("Usage: {program} [options]");
    print!("{}", opts.usage(&brief));
}

fn check_free_opts(free: &[String]) -> Result<()> {
    ensure!(free.is_empty(), "Unrecognized option: {}", free[0]);

    Ok(())
}

pub fn entry_point(argv: &[impl AsRef<str>]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let program = argv[0].as_ref();
    let args: Vec<&str> = argv[1..].iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    let matches = opts.parse(args).context(USAGE_HINT)?;
    check_free_opts(&matches.free).context(USAGE_HINT)?;

    if matches.opt_present("h") {
        print_help(program, opts);
        return Ok(());
    }

    bail!("not implemented");
}
