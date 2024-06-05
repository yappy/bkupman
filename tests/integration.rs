use anyhow::Result;
use bkupman;

// Return argv[0] (program name)
// Panic unless argc == 1
fn get_argv0() -> String {
    let mut args = std::env::args();
    let argv0 = args.next().unwrap();
    assert_eq!(args.next(), None);

    argv0
}

#[test]
fn print_help() -> Result<()> {
    let argv = [&get_argv0(), "--help"];
    bkupman::entry_point(&argv)?;

    let argv = [&get_argv0(), "-h"];
    bkupman::entry_point(&argv)?;

    Ok(())
}
