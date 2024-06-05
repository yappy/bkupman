use anyhow::Result;

fn main() -> Result<()> {
    let argv: Vec<String> = std::env::args().collect();
    bkupman::entry_point(&argv)?;

    Ok(())
}
