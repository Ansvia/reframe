#![allow(dead_code)]

use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use zip::ZipArchive;

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn extract_zip<P: AsRef<Path>>(zip_path: P, out_dir: P) -> io::Result<()> {
    let fin = File::open(&zip_path).unwrap_or_else(|e| {
        panic!(
            "Cannot open zip file `{}`. {}",
            zip_path.as_ref().display(),
            e
        )
    });
    let mut archive = ZipArchive::new(fin)?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .unwrap_or_else(|e| panic!("Cannot get zip entry index `{}`: {}", i, e));
        let outpath = out_dir.as_ref().join(file.mangled_name());

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (&*file.name()).ends_with('/') {
            debug!(
                "File {} extracted to \"{}\"",
                i,
                outpath.as_path().display()
            );
            fs::create_dir_all(&outpath)?;
        } else {
            debug!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

/// Download data from internet.
pub async fn download<P: AsRef<Path>>(
    url: &str,
    out_dir: P,
    out_file_name: &str,
) -> io::Result<()> {
    fs::create_dir_all(&out_dir)?;

    let out_path = out_dir.as_ref().join(out_file_name);

    {
        let mut fw = File::create(&out_path)?;

        debug!("downloading {} ...", url);

        let resp = reqwest::get(url).await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("cannot clone from github: {} (error: {})", url, e),
            )
        })?;

        let mut content = std::io::Cursor::new(resp.bytes().await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("cannot read data from github: {} (error: {})", url, e),
            )
        })?);

        std::io::copy(&mut content, &mut fw)?;
    }

    // extract zip file
    let zip_file = out_path.clone();

    debug!(
        "extracting `{}` to `{}` ...",
        &zip_file.display(),
        &out_dir.as_ref().display()
    );

    if extract_zip(zip_file, out_dir.as_ref().to_path_buf()).is_err() {
        Err(io::Error::from(io::ErrorKind::NotFound))?;
    }

    Ok(())
}

#[inline]
pub fn file_pattern_match<S>(file_name: &str, patts: &[S]) -> bool
where
    S: AsRef<str>,
{
    let p = Path::new(file_name);
    if let Some(ext) = p.extension() {
        for patt in patts {
            if patt.as_ref().contains("*.") {
                if ext == &patt.as_ref()[2..patt.as_ref().len()] {
                    return true;
                }
            } else if file_name == patt.as_ref() {
                return true;
            }
        }
    } else {
        for patt in patts {
            if file_name == patt.as_ref() {
                return true;
            }
        }
    }
    false
}

#[inline]
pub fn path_to_relative<P: AsRef<Path>>(path: P, root: P) -> PathBuf {
    path.as_ref()
        .strip_prefix(&root)
        .unwrap_or_else(|_| {
            panic!(
                "Cannot get relative path for `{}` from `{}`",
                path.as_ref().display(),
                root.as_ref().display()
            )
        })
        .to_owned()
}

/// komparasi versi, hanya support max 3 level.
pub fn compare_version(version_a: &str, version_b: &str) -> i32 {
    #[inline(always)]
    fn split(v: &str) -> (i32, i32, i32) {
        let s: Vec<&str> = v.split('.').collect();
        let s1 = if !s.is_empty() {
            s[0].parse::<i32>().unwrap_or(0) + 1000
        } else {
            0
        };

        let s2 = if s.len() > 1 {
            s[1].parse::<i32>().unwrap_or(0) + 100
        } else {
            0
        };

        let s3 = if s.len() > 2 {
            s[2].parse::<i32>().unwrap_or(0) + 10
        } else {
            0
        };

        (s1, s2, s3)
    }

    let (s1, s2, s3) = split(version_a);
    let (y1, y2, y3) = split(version_b);

    let v_a: i32 = format!("{}{}{}", y1, y2, y3).parse().unwrap_or(0);
    let v_b: i32 = format!("{}{}{}", s1, s2, s3).parse().unwrap_or(0);

    let rv = v_a - v_b;

    if rv > 1 {
        1
    } else if rv < -1 {
        -1
    } else {
        rv
    }
}

/// Mendapatkan timestamp waktu terkini.
pub fn get_current_time_millis() -> u128 {
    let start = SystemTime::now();
    let since_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("cannot get time duration since epoch");
    u128::from(since_epoch.as_secs()) * 1000 + u128::from(since_epoch.subsec_millis())
}

const SOURCES_URL: &'static str =
    "https://raw.githubusercontent.com/Ansvia/reframe/master/SOURCES.md";
lazy_static! {
    static ref RE_REFRAME_URL: Regex =
        Regex::new(r"\* \[(\w+/[a-zA-Z0-9-.]*?)]\(https://github\.com/.*?.rf\) - (.*)").unwrap();
}

pub async fn get_available_sources() -> io::Result<Vec<(String, String)>> {
    let a = get_current_time_millis();
    let url = format!("{}?a={}", SOURCES_URL, a);
    log::debug!("querying sources from: {}", url);
    let resp = reqwest::get(url).await.map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Cannot query sources: {} (error: {})", SOURCES_URL, e),
        )
    })?;

    let text = resp.text().await.map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Cannot read sources from: {} (error: {})", SOURCES_URL, e),
        )
    })?;

    log::debug!("sources: {}", text);

    let texts = text
        .split("\n")
        .filter(|s| RE_REFRAME_URL.is_match(s))
        .map(|s| match RE_REFRAME_URL.captures(s) {
            Some(x) => Some((x.get(1), x.get(2))),
            None => None,
        })
        .flatten()
        .map(|(a, b)| {
            (
                a.map(|a| a.as_str()).unwrap_or(""),
                b.map(|b| b.as_str()).unwrap_or(""),
            )
        })
        .map(|(name, description)| (name.to_string(), description.to_string()))
        .collect();

    Ok(texts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_pattern_match() {
        let patts = ["README.md", "*.iml", ".packages"];
        assert_eq!(file_pattern_match("test.iml", &patts), true);
        assert_eq!(file_pattern_match("test.txt", &patts), false);
        assert_eq!(file_pattern_match("README.md", &patts), true);
        assert_eq!(file_pattern_match("README.txt", &patts), false);
        assert_eq!(file_pattern_match(".packages", &patts), true);
        assert_eq!(file_pattern_match(".iml", &patts), false);
    }

    #[test]
    fn test_path_to_relative() {
        let root = "/tmp";
        assert_eq!(
            &format!("{}", path_to_relative("/tmp/satu/dua", &root).display()),
            "satu/dua"
        );
        assert_eq!(
            &format!(
                "{}",
                path_to_relative("/tmp/satu/dua/tiga", &root).display()
            ),
            "satu/dua/tiga"
        );
    }

    #[test]
    fn test_compare_version() {
        assert_eq!(compare_version("0.0.1", "0.0.2"), 1);
        assert_eq!(compare_version("0.0.3", "0.0.1"), -1);
        assert_eq!(compare_version("1.2.3", "1.2.3"), 0);
        assert_eq!(compare_version("0.2.1", "0.1.1"), -1);
        assert_eq!(compare_version("0.2.1", "3.1.1"), 1);
        assert_eq!(compare_version("4.2.1", "4.1.1"), -1);
        assert_eq!(compare_version("0.0.0", "0.0.12"), 1);
        assert_eq!(compare_version("0.1.2", "0.1.2"), 0);
        assert_eq!(compare_version("1.0.0", "0.0.0"), -1);
        assert_eq!(compare_version("1.1.2", "1.2.0"), 1);
        assert_eq!(compare_version("0.1.8", "0.2.0"), 1);
        assert_eq!(compare_version("1", "2"), 1);
    }

    #[test]
    fn test_compare_version_empty() {
        assert_eq!(compare_version("0.2.3", ""), -1);
    }

    #[test]
    fn test_get_current_time_millis() {
        assert_ne!(get_current_time_millis(), 0);
    }
}
