use std::path::Path;

use anyhow::{Context, Result};
use dialoguer::Password;
use getopts::Options;
use log::info;

use super::Config;
use crate::{
    commands::{Crypt, CONFIG_FILE_NAME},
    cryptutil, util,
};

fn process_genkey(mut config: Config) -> Result<Option<Config>> {
    info!("Generate a new encrypt/decrypt key");

    let password = Password::new()
        .with_prompt("Passphrase")
        .allow_empty_password(true)
        .with_confirmation("Input again", "Passphrase mismatch")
        .interact()?;

    let (salt, m_cost, t_cost, p_cost, key) = cryptutil::aeskey_new_from_password(&password);
    config.crypt = Crypt::Argon2 {
        key: Some(key),
        salt,
        m_cost,
        t_cost,
        p_cost,
    };

    info!("New salt and key created: {}", config.crypt);
    info!(
        "New AES en/decrypt key created: (Saved into {})",
        CONFIG_FILE_NAME
    );

    Ok(Some(config))
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const DESC: &str = "Generate encryption key";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    if crate::util::find_option(&args, &["-h", "--help"]) {
        println!("{}", util::create_help(cmd, DESC, &opts));
        return Ok(());
    }
    let _matches = opts.parse(args).context(USAGE_HINT)?;

    super::process_with_config_lock(basedir, |_dirpath, config| process_genkey(config))
}
