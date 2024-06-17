use std::io::SeekFrom;

use anyhow::{anyhow, ensure, Result};
use getopts::Options;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

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

pub fn size_to_human_readable(size: u64) -> String {
    if size < 1024 {
        return format!("{size} B");
    }

    let mut unit = 1024.0;
    let fsize = size as f64;
    let num = fsize / unit;
    if num < 1024.0 {
        return format!("{num:.1} KiB");
    }

    unit *= 1024.0;
    let num = fsize / unit;
    if num < 1024.0 {
        return format!("{num:.1} MiB");
    }

    unit *= 1024.0;
    let num = fsize / unit;
    if num < 1024.0 {
        return format!("{num:.1} GiB");
    }

    unit *= 1024.0;
    let num = fsize / unit;
    format!("{num:.1} TiB")
}

// 128 bit
pub const MD5LEN: usize = 16;
pub const MD5STRLEN: usize = 32;

pub fn str_to_md5(s: &str) -> Result<[u8; MD5LEN]> {
    ensure!(s.len() == MD5STRLEN);

    let mut hash = [0; MD5LEN];
    for (i, x) in hash.iter_mut().enumerate() {
        let b = u8::from_str_radix(&s[(i * 2)..=(i * 2 + 1)], 16)?;
        *x = b;
    }

    Ok(hash)
}

pub fn md5_to_str(md5: &[u8]) -> String {
    assert_eq!(md5.len(), MD5LEN);

    let mut result = String::with_capacity(MD5STRLEN);
    for &b in md5 {
        result += &format!("{:0>2x}", b);
    }

    result
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

pub async fn read_fully(file: &mut tokio::fs::File, buf: &mut [u8]) -> Result<usize> {
    let mut cur = 0usize;
    while cur < buf.len() {
        let rsize = file.read(&mut buf[cur..]).await?;
        if rsize == 0 {
            // EOF
            break;
        }
        cur += rsize;
    }

    Ok(cur)
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

    #[test]
    fn test_size_to_human_readable() {
        for size in 0..1024 {
            let s = size_to_human_readable(size);
            assert_eq!(s, format!("{size} B"));
        }
        assert_eq!(size_to_human_readable(1024), "1.0 KiB");
        assert_eq!(size_to_human_readable(1024 * 1024), "1.0 MiB");
        assert_eq!(size_to_human_readable(1024 * 1024 * 1024), "1.0 GiB");
        assert_eq!(size_to_human_readable(1024 * 1024 * 1024 * 1024), "1.0 TiB");
    }

    #[test]
    fn test_md5() -> Result<()> {
        let s = "0123456789abcdef0123456789abcdef";
        let b = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];

        let x = str_to_md5(s)?;
        assert_eq!(x, b);

        let y = md5_to_str(&b);
        assert_eq!(y, s);

        Ok(())
    }
}
