use anyhow::Result;
use tempdir::TempDir;

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

#[test]
fn init() -> Result<()> {
    let dir = TempDir::new("example")?;
    let dirstr = dir.path().to_str().unwrap();

    let argv = [&get_argv0(), "-C", dirstr, "init"];
    bkupman::entry_point(&argv)?;

    Ok(())
}
