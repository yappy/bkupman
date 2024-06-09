use std::{fs, path::Path, usize};

use anyhow::Result;
use tempdir::TempDir;

// Return argv[0] (program name)
fn get_argv0() -> String {
    std::env::args().next().unwrap()
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
    let dir = TempDir::new("bkupman-test")?;
    let dirstr = dir.path().to_str().unwrap();

    let argv = [&get_argv0(), "-C", dirstr, "init"];
    bkupman::entry_point(&argv)?;

    Ok(())
}

fn create_files(dirpath: &Path, count: usize) -> Result<()> {
    for i in 0..count {
        let path = dirpath.join(format!("file{i}"));
        fs::write(&path, "")?;
        println!("{}", path.to_string_lossy());
    }
    Ok(())
}

#[test]
fn inbox_many() -> Result<()> {
    let dir = TempDir::new("bkupman-test")?;
    let dirpath = dir.path();
    let dirstr = dirpath.to_str().unwrap();

    let argv = [&get_argv0(), "-C", dirstr, "init"];
    bkupman::entry_point(&argv)?;

    create_files(&dirpath.join("inbox"), 1000)?;

    let argv = [&get_argv0(), "-C", dirstr, "inbox"];
    bkupman::entry_point(&argv)?;

    Ok(())
}
