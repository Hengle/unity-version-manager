use std::path::Path;
use std::path::PathBuf;
use std::fs;
use unity::Version;
use std::str::FromStr;
use std::io;

const UNITY_INSTALL_LOCATION: &'static str = "/Applications";

pub struct Installation {
    pub version: Version,
    pub path: PathBuf,
}

pub fn list() -> io::Result<Vec<Installation>> {
    let install_location = Path::new(UNITY_INSTALL_LOCATION);
    let mut versions: Vec<Installation> = Vec::new();
    let files = fs::read_dir(install_location)?;
    for entry in files {
        let entry = entry.expect("Error while reading Directory entry.");
        let file_name_s = entry.file_name();

        let file_name = file_name_s.to_str().expect("Error while reading filename");
        if file_name.starts_with("Unity-") {
            if let Ok(v) = Version::from_str(file_name) {
                versions.push(Installation{version: v, path: entry.path()})
            }
        }
    }
    //versions.sort();
    Ok(versions)
}
