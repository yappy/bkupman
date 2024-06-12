use anyhow::{anyhow, ensure, Result};
use getopts::Options;

/// The library getopts workaround.
///
/// If required option is added and that option is missing,
/// the parser will fail to parse `--help` or `--version` command.
///
/// This function finds options manually.
pub fn find_option(args: &[impl AsRef<str>], optstrs: &[&str]) -> bool {
    for arg in args {
        let arg = arg.as_ref();
        if optstrs.iter().any(|&s| arg == s) {
            return true;
        }
    }
    false
}

pub fn create_help(program: &str, desc: &str, opts: &Options) -> String {
    let brief = format!("Usage: {program} [options]\n{desc}");
    opts.usage(&brief)
}

pub fn parse_size(s: &str) -> Result<u64> {
    ensure!(s.is_ascii(), "string is not ascii");
    ensure!(!s.is_empty(), "string is empty");

    let last = &s[s.len() - 1..s.len()];
    let unit: u64 = match last {
        "k" | "K" => 1u64 << 10,
        "m" | "M" => 1u64 << 20,
        "g" | "G" => 1u64 << 30,
        "t" | "T" => 1u64 << 40,
        _ => 1,
    };

    let numstr = if unit != 1 {
        &s[..s.len() - 1]
    } else {
        &s[..s.len()]
    };
    let num: u64 = numstr.parse()?;

    num.checked_mul(unit).ok_or_else(|| anyhow!("overflow"))
}

pub fn seed64() -> u64 {
    rand::random()
}

pub fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;

    x
}

pub fn xorshift64_fill(v: &mut [u8], state: u64) -> u64 {
    assert!(v.len() % 8 == 0);

    let mut x = state;
    for i in (0..v.len()).step_by(8) {
        x = xorshift64(x);
        for (k, &x) in x.to_ne_bytes().iter().enumerate() {
            v[i + k] = x;
        }
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_option() -> Result<()> {
        assert!(find_option(&["-h", "a"], &["-h", "--help"]));
        assert!(find_option(&["a", "--help"], &["-h", "--help"]));
        assert!(!find_option(&([] as [&str; 0]), &["-h", "--help"]));
        assert!(!find_option(&["a", "b", "c"], &["-h", "--help"]));

        Ok(())
    }

    #[test]
    fn test_parse_size() -> Result<()> {
        assert_eq!(0, parse_size("0").unwrap());
        assert_eq!(1, parse_size("1").unwrap());
        assert_eq!(u64::MAX, parse_size(&u64::MAX.to_string()).unwrap());

        assert_eq!(12345, parse_size("12345").unwrap());
        assert_eq!(12345 << 10usize, parse_size("12345k").unwrap());
        assert_eq!(12345 << 10usize, parse_size("12345K").unwrap());
        assert_eq!(12345 << 20usize, parse_size("12345m").unwrap());
        assert_eq!(12345 << 20usize, parse_size("12345M").unwrap());
        assert_eq!(12345 << 30usize, parse_size("12345g").unwrap());
        assert_eq!(12345 << 30usize, parse_size("12345G").unwrap());
        assert_eq!(12345 << 40usize, parse_size("12345t").unwrap());
        assert_eq!(12345 << 40usize, parse_size("12345T").unwrap());

        assert!(parse_size("0x123").is_err());
        assert!(parse_size("123x").is_err());
        assert!(parse_size(&(usize::MAX.to_string() + "0")).is_err());
        assert!(parse_size(&(usize::MAX.to_string() + "k")).is_err());

        Ok(())
    }
}
