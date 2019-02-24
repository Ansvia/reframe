use zip::ZipArchive;

use std::{
    fs::{self, File},
    io,
    path::Path,
};

fn extract_zip<P: AsRef<Path>>(zip_path: P, out_dir: P) -> io::Result<()> {
    let fin = File::open(&zip_path)
        .unwrap_or_else(|_| panic!("Cannot open zip file `{}`", zip_path.as_ref().display()));
    let mut archive = ZipArchive::new(fin)?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .unwrap_or_else(|e| panic!("Cannot get zip entry index `{}`: {}", i, e));
        let outpath = out_dir.as_ref().join(file.sanitized_name());

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

/// Download data from repo
pub fn download<P: AsRef<Path>>(url: &str, out_dir: P) -> io::Result<()> {
    fs::create_dir_all(&out_dir)?;

    let out_path = out_dir.as_ref().join("master.zip");

    {
        let mut w = File::create(&out_path)?;

        debug!("downloading {} ...", url);

        reqwest::get(url)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("cannot clone from github: {} (error: {})", url, e),
                )
            })?
            .copy_to(&mut w)
            .unwrap_or_else(|_| panic!("cannot store data to `{}`", out_path.display()));
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
