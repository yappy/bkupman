use std::{path::Path, str::FromStr};

use anyhow::{ensure, Context, Result};
use dialoguer::Password;
use getopts::Options;
use log::info;
use strum::{EnumMessage, IntoEnumIterator};

use crate::{
    commands::{Aes128GcmArgon2Param, Config, CryptType, CONFIG_FILE_NAME},
    cryptutil, util,
};

fn genkey_plaintext(mut config: Config) -> Result<Option<Config>> {
    config.crypt = CryptType::PlainText;

    Ok(Some(config))
}

fn genkey_aes(mut config: Config) -> Result<Option<Config>> {
    info!("Generate a new encrypt/decrypt key");

    let password = Password::new()
        .with_prompt("Passphrase")
        .allow_empty_password(true)
        .with_confirmation("Input again", "Passphrase mismatch")
        .interact()?;

    let (salt, m_cost, t_cost, p_cost, key) = cryptutil::aeskey_new_from_password(&password);
    config.crypt = CryptType::Aes128GcmArgon2 {
        key: Some(key),
        argon2: Aes128GcmArgon2Param {
            salt,
            m_cost,
            t_cost,
            p_cost,
        },
    };

    info!("New salt and key created: {}", config.crypt);
    info!(
        "New AES en/decrypt key created: (Saved into {})",
        CONFIG_FILE_NAME
    );

    Ok(Some(config))
}

fn process_key(config: Config, ctype: Option<&str>) -> Result<Option<Config>> {
    if let Some(ctype) = ctype {
        let ctype = CryptType::from_str(ctype).with_context(|| {
            info!("{}", crypt_type_help());
            format!("Invalid crypt type - {ctype}")
        })?;
        match ctype {
            CryptType::PlainText => genkey_plaintext(config),
            CryptType::Aes128GcmArgon2 { .. } => genkey_aes(config),
        }
    } else {
        // print status
        info!("Current status: {}", config.crypt);
        Ok(None)
    }
}

fn crypt_type_help() -> String {
    let mut res = "Supported types:\n".to_string();

    for t in CryptType::iter() {
        let name = t.get_serializations()[0];
        let msg = t.get_message().unwrap_or_default();
        let detail = t.get_detailed_message().unwrap_or_default();

        res += &format!("{name}\n  {msg}\n  {detail}\n");
    }

    res
}

pub fn entry(basedir: &Path, cmd: &str, args: &[String]) -> Result<()> {
    const DESC: &str = "Show/Generate encryption key";
    const USAGE_HINT: &str = "--help or -h to show usage";
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");

    if util::find_option(&args, &["-h", "--help"]) {
        println!("{}", util::create_help(cmd, DESC, &opts, Some("[TYPE]")));
        println!("{}", crypt_type_help());
        return Ok(());
    }
    let matches = opts.parse(args).context(USAGE_HINT)?;

    ensure!(matches.free.len() < 2, "Too much arguments");
    let ctype = matches.free.first().map(|s| s.as_str());

    super::process_with_config_lock(basedir, |_dirpath, config| process_key(config, ctype))
}
