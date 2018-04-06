extern crate regex;

#[cfg(test)]
#[macro_use]
extern crate proptest;
#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate tempdir;

#[macro_export]
macro_rules! cargo_version {
    // `()` indicates that the macro takes no argument.
    () => (
        // The macro will expand into the contents of this block.
        format!("{}.{}.{}{}",
          env!("CARGO_PKG_VERSION_MAJOR"),
          env!("CARGO_PKG_VERSION_MINOR"),
          env!("CARGO_PKG_VERSION_PATCH"),
          option_env!("CARGO_PKG_VERSION_PRE").unwrap_or(""));
    )
}

pub mod unity;

pub use self::unity::list_installations;
pub use self::unity::current_installation;

pub use self::unity::Installation;
pub use self::unity::CurrentInstallation;
pub use self::unity::Version;

use std::io;
use std::fs;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::os::unix;
use std::str::FromStr;
use std::convert::AsRef;

pub fn is_active(version: &Version) -> bool {
    if let Ok(current) = current_installation() {
        current.version() == version
    } else {
        false
    }
}

pub fn find_installation(version: &Version) -> io::Result<Installation> {
    let mut installations = list_installations()?;
    installations
        .find(|i| i.version() == version)
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Unable to find Unity version {}", version),
        ))
}

pub fn activate(ref installation: Installation) -> io::Result<()> {
    let active_path = Path::new("/Applications/Unity");
    if active_path.exists() {
        fs::remove_file(active_path)?;
    }
    unix::fs::symlink(installation.path(), active_path)?;
    Ok(())
}

fn get_project_version<P: AsRef<Path>>(base_dir: P) -> io::Result<PathBuf> {
    let project_version = base_dir
        .as_ref()
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    match project_version.exists() {
        true => Ok(project_version),
        false => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "directory {} is not a Unity project",
                base_dir.as_ref().display()
            ),
        )),
    }
}

pub fn detect_unity_project_dir(dir: &Path, recur: bool) -> io::Result<PathBuf> {
    let error = Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Unable to find a Unity project",
    ));

    if dir.is_dir() {
        if get_project_version(dir).is_ok() {
            return Ok(dir.to_path_buf());
        } else if !recur {
            return error;
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let f = detect_unity_project_dir(&path, true);
                if f.is_ok() {
                    return f;
                }
            }
        }
    }
    error
}

pub fn dectect_project_version(project_path: &Path, recur: Option<bool>) -> io::Result<Version> {
    let project_version = detect_unity_project_dir(project_path, recur.unwrap_or(false))
        .and_then(get_project_version)?;

    let mut file = File::open(project_version)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Version::from_str(&contents)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Can't parse Unity version"))
}