mod commands;

use std::{collections::BTreeMap, sync::OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use getopts::Options;

type CommandFunc = Box<dyn Fn(&str, &[String]) -> Result<()> + Send + Sync + 'static>;

fn dispatch_table() -> &'static BTreeMap<&'static str, CommandFunc> {
    static TABLE: OnceLock<BTreeMap<&str, CommandFunc>> = OnceLock::new();

    TABLE.get_or_init(|| {
        let mut table: BTreeMap<&'static str, CommandFunc> = BTreeMap::new();
        table.insert("init", Box::new(commands::init::entry));
        table.insert("inbox", Box::new(commands::inbox::entry));
        table
    })
}

fn print_help_subcommands(program: &str, opts: &Options) {
    let brief = format!("Usage: {program} [options]");
    println!("{}", opts.usage(&brief));

    let table = dispatch_table();
    println!("Subcommands:");
    for &key in table.keys() {
        println!("    {key}");
    }
}

fn dispatch_subcommand(argv: &[String]) -> Result<()> {
    let table = dispatch_table();
    let argv0: &str = &argv[0];
    let func = table
        .get(argv0)
        .ok_or(anyhow!("Subcommand not found: {argv0}"))?;

    func(argv0, &argv[1..])
}

pub fn entry_point(argv: &[impl AsRef<str>]) -> Result<()> {
    const USAGE_HINT: &str = "--help or -h to show usage";
    let program = argv[0].as_ref();
    let args: Vec<&str> = argv[1..].iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    // free argument means starting of sub command
    opts.parsing_style(getopts::ParsingStyle::StopAtFirstFree);
    // main command arguments
    opts.optflag("h", "help", "Print this help");
    opts.optopt(
        "C",
        "directory",
        "Change working directory at first",
        "DIRECTORY",
    );

    let matches = opts.parse(args).context(USAGE_HINT)?;

    // process main arguments
    if matches.opt_present("h") {
        print_help_subcommands(program, &opts);
        return Ok(());
    }
    if let Some(dir) = matches.opt_str("C") {
        println!("Set current directory: {dir}");
        println!();
        std::env::set_current_dir(dir).context("Changing directory failed")?;
    }

    if !matches.free.is_empty() {
        dispatch_subcommand(&matches.free)
    } else {
        print_help_subcommands(program, &opts);
        bail!("Subcommand not specified")
    }
}
