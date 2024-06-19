mod commands;
mod util;

use std::{fs::File, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use getopts::Options;
use log::{error, info, LevelFilter};
use simplelog::{
    ColorChoice, CombinedLogger, Config, ConfigBuilder, SharedLogger, SimpleLogger, TermLogger,
    TerminalMode, WriteLogger,
};

fn initialize_logger(test_mode: bool, log_files: Vec<String>) -> Result<()> {
    if test_mode {
        let _ = SimpleLogger::init(LevelFilter::Trace, Default::default());
        // ignore error (set once)
        return Ok(());
    }

    let config = create_log_config();

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];
    // terminal
    loggers.push(TermLogger::new(
        LevelFilter::Trace,
        config.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ));
    // file
    for file in log_files.iter() {
        loggers.push(WriteLogger::new(
            LevelFilter::Info,
            config.clone(),
            File::create(file).with_context(|| format!("Failed to open log file: {file}"))?,
        ));
    }

    // fails only if logger is already set
    CombinedLogger::init(loggers).unwrap();
    info!("Log setup");

    Ok(())
}

fn create_log_config() -> Config {
    ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .set_time_format_rfc2822()
        .build()
}

fn print_help_subcommands(program: &str, opts: &Options) {
    let brief = format!("Usage: {program} [options...] SUBCMD [options...]");
    println!("{}", opts.usage(&brief));

    let table = commands::dispatch_table();
    println!("Subcommands:");
    for &key in table.keys() {
        println!("    {key}");
    }
}

fn dispatch_subcommand(basedir: impl AsRef<Path>, argv: &[String]) -> Result<()> {
    assert!(!argv.is_empty());

    let table = commands::dispatch_table();
    let argv0: &str = &argv[0];
    let func = table
        .get(argv0)
        .ok_or(anyhow!("Subcommand not found: {argv0}"))?;

    func(basedir.as_ref(), argv0, &argv[1..])
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
        "Set root directory for working",
        "DIRECTORY",
    );
    opts.optflag("t", "test-mode", "Test mode (disable log)");
    opts.optmulti("l", "log", "Add log file", "LOGFILE");

    let matches = opts.parse(args).context(USAGE_HINT)?;

    // process main arguments
    if matches.opt_present("h") {
        print_help_subcommands(program, &opts);
        return Ok(());
    }
    if matches.free.is_empty() {
        print_help_subcommands(program, &opts);
        bail!("Subcommand not specified");
    }
    let test_mode = matches.opt_present("t");

    let log_files = matches.opt_strs("l");
    initialize_logger(test_mode, log_files)?;

    let work_main = || {
        let basedir = if let Some(dir) = matches.opt_str("C") {
            info!("Set base directory: {dir}");

            dir
        } else {
            ".".to_string()
        };

        dispatch_subcommand(basedir, &matches.free)
    };

    match work_main() {
        Ok(()) => {
            info!("Completed successfully");
            Ok(())
        }
        Err(err) => {
            // don't return from main()
            error!("Command failed");
            error!("{:#}", err);
            std::process::exit(1);
        }
    }
}
