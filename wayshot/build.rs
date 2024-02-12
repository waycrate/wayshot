extern crate flate2;
use eyre::{ContextCompat, Result};
use flate2::{write::GzEncoder, Compression};
use std::{
    fs::{read_dir, File, OpenOptions},
    io::{copy, BufReader, ErrorKind},
    path::Path,
    process::{Command, Stdio},
};

fn main() -> Result<()> {
    if let Err(e) = Command::new("scdoc")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let ErrorKind::NotFound = e.kind() {
            return Ok(());
        }
    }

    // We just append "out" so it's easy to find all the scdoc output later in line 38.
    let man_pages: Vec<(String, String)> = read_and_replace_by_ext("./docs", ".scd", ".out")?;
    for man_page in man_pages {
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .open(Path::new(&man_page.1))?;
        _ = Command::new("scdoc")
            .stdin(Stdio::from(File::open(man_page.0)?))
            .stdout(output)
            .spawn();
    }

    // Gzipping the man pages
    let scdoc_output_files: Vec<(String, String)> =
        read_and_replace_by_ext("./docs", ".out", ".gz")?;
    for scdoc_output in scdoc_output_files {
        let mut input = BufReader::new(File::open(scdoc_output.0)?);
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .open(Path::new(&scdoc_output.1))?;
        let mut encoder = GzEncoder::new(output, Compression::default());
        copy(&mut input, &mut encoder)?;
        encoder.finish()?;
    }

    Ok(())
}

fn read_and_replace_by_ext(
    path: &str,
    search: &str,
    replace: &str,
) -> Result<Vec<(String, String)>> {
    let mut files: Vec<(String, String)> = Vec::new();
    for path in read_dir(path)? {
        let path = path?;
        if path.file_type()?.is_dir() {
            continue;
        }

        if let Some(file_name) = path.path().to_str() {
            if *path
                .path()
                .extension()
                .wrap_err_with(|| format!("no extension found for {}", path.path().display()))?
                .to_string_lossy()
                != search[1..]
            {
                continue;
            }

            let file = file_name.replace(search, replace);
            files.push((file_name.to_string(), file));
        }
    }
    Ok(files)
}
