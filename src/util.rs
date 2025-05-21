#[cfg(feature = "to_pub")]
use std::{env, fs, io::Write, ops::Deref, path::PathBuf};

#[cfg(all(feature = "to_pub", target_os = "windows"))]
/// Tries to find and create a the file to store camera index information in.
///
/// It makes no guarantees about the contents of the file before this function was called, and it uses
/// [`fs::File::create`] to ensure the file exists and is empty. Any remaining data from previous sessions will be
/// deleted due to the truncation of calling `create` on an existing file.
pub fn find_camfile() -> Option<PathBuf> {
    let pubdir = PathBuf::from(env::var("PUBLIC").unwrap_or("C:/Users/Public".into()));
    if !pubdir.exists() {
        if fs::create_dir_all(&pubdir).is_err() {
            eprintln!("Public directory does not exist! Failed to recursively create {}", pubdir.to_string_lossy());
            return None;
        }
    }
    // At this point we know the directory exists. If it didn't, we just created it.
    let camfile = pubdir.canonicalize().unwrap();
    let camfile = camfile.join("shmem_camindices.txt");
    if fs::File::create(&camfile).is_err() {
        eprintln!(
            "Pubdir exists at {} but the camfile could not be created at {}",
            pubdir.to_string_lossy(),
            camfile.to_string_lossy()
        );
        return None;
    }
    Some(camfile)
}

#[cfg(all(feature = "to_pub", target_os = "windows"))]
/// Write the existing camera indices to a camfile.
///
/// If no camfile is provided, this function will call [`find_camfile`] to ensure its creation and truncation.
/// If a non-empty camfile is provided, the cameras from this run will be appended at the bottom.
pub fn write_camfile(camnames: impl Deref<Target = str>, camfile: Option<&PathBuf>) {
    if let Some(outfile) = match camfile {
        Some(f) => Some(f.clone()),
        None => find_camfile(),
    } {
        let fh = fs::File::options().append(true).create(true).truncate(false).open(&outfile);
        if fh.is_err() {
            eprintln!("Cannot open or create {}! Skipping the public file write!", outfile.to_string_lossy());
            return;
        }
        let mut fh = fh.unwrap();
        fh.write(camnames.as_bytes()).unwrap();
        fh.sync_data().unwrap()
    }
}
