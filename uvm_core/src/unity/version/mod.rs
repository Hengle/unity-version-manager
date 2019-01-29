use regex::Regex;
use semver;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::convert::{AsMut, AsRef, From};
use std::fmt;
use std::result;
use std::str::FromStr;
use unity::Installation;
use std::io;
mod hash;
pub mod manifest;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod other;
#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "macos")]
use self::mac as sys;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use self::other as sys;
#[cfg(target_os = "windows")]
use self::win as sys;

pub use self::hash::all_versions;
pub use self::sys::read_version_from_path;

#[derive(PartialEq, Eq, Ord, Hash, Debug, Clone, Copy, Deserialize)]
pub enum VersionType {
    Alpha,
    Beta,
    Patch,
    Final,
}

impl PartialOrd for VersionType {
    fn partial_cmp(&self, other: &VersionType) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Eq, Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct Version {
    base: semver::Version,
    release_type: VersionType,
    revision: u64,
    hash: Option<String>,
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        self.base
            .cmp(&other.base)
            .then(self.release_type.cmp(&other.release_type))
            .then(self.revision.cmp(&other.revision))
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Version {
    pub fn new(
        major: u64,
        minor: u64,
        patch: u64,
        release_type: VersionType,
        revision: u64,
    ) -> Version {
        let base = semver::Version::new(major, minor, patch);
        Version {
            base,
            release_type,
            revision,
            hash: None,
        }
    }

    pub fn a(major: u64, minor: u64, patch: u64, revision: u64) -> Version {
        let base = semver::Version::new(major, minor, patch);
        Version {
            base,
            release_type: VersionType::Alpha,
            revision,
            hash: None,
        }
    }

    pub fn b(major: u64, minor: u64, patch: u64, revision: u64) -> Version {
        let base = semver::Version::new(major, minor, patch);
        Version {
            base,
            release_type: VersionType::Beta,
            revision,
            hash: None,
        }
    }

    pub fn p(major: u64, minor: u64, patch: u64, revision: u64) -> Version {
        let base = semver::Version::new(major, minor, patch);
        Version {
            base,
            release_type: VersionType::Patch,
            revision,
            hash: None,
        }
    }

    pub fn f(major: u64, minor: u64, patch: u64, revision: u64) -> Version {
        let base = semver::Version::new(major, minor, patch);
        Version {
            base,
            release_type: VersionType::Final,
            revision,
            hash: None,
        }
    }

    pub fn release_type(&self) -> &VersionType {
        &self.release_type
    }

    pub fn version_hash(&self) -> std::io::Result<String> {
        hash::hash_for_version(self)
    }

    pub fn major(&self) -> u64 {
        self.base.major
    }

    pub fn minor(&self) -> u64 {
        self.base.minor
    }

    pub fn patch(&self) -> u64 {
        self.base.patch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn base(&self) -> &semver::Version {
        &self.base
    }

    pub fn as_semver(&self) -> semver::Version {
        let mut v = self.base.clone();
        if self.release_type != VersionType::Final {
            v.pre.push(semver::Identifier::AlphaNumeric(self.release_type.to_string()));
            v.pre.push(semver::Identifier::Numeric(self.revision));
        }
        v
    }
}

impl From<(u64, u64, u64, u64)> for Version {
    fn from(tuple: (u64, u64, u64, u64)) -> Version {
        let (major, minor, patch, revision) = tuple;
        Version::f(major, minor, patch, revision)
    }
}

impl fmt::Display for VersionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            match *self {
                VersionType::Final => write!(f, "final"),
                VersionType::Patch => write!(f, "patch"),
                VersionType::Beta => write!(f, "beta"),
                VersionType::Alpha => write!(f, "alpha"),
            }
        } else {
            match *self {
                VersionType::Final => write!(f, "f"),
                VersionType::Patch => write!(f, "p"),
                VersionType::Beta => write!(f, "b"),
                VersionType::Alpha => write!(f, "a"),
            }
        }
    }
}

impl Default for VersionType {
    fn default() -> VersionType {
        VersionType::Final
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.base,
            self.release_type.to_string(),
            self.revision
        )
    }
}

impl AsRef<Version> for Version {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsMut<Version> for Version {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

error_chain! {
    types {
        UvmVersionError, UvmVersionErrorKind, ResultExt, Result;
    }

    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
    }

    errors {
        NotAUnityInstalltion(path: String) {
            description("path is not a unity installtion"),
            display("Provided Path: '{}' is not a Unity installation.", path),
        }
    }
}

impl FromStr for Version {
    type Err = UvmVersionError;

    fn from_str(s: &str) -> Result<Self> {
        let version_pattern =
            Regex::new(r"([0-9]{1,4})\.([0-9]{1,4})\.([0-9]{1,4})(f|p|b|a)([0-9]{1,4})").unwrap();
        match version_pattern.captures(s) {
            Some(caps) => {
                let major: u64 = caps.get(1).map_or("0", |m| m.as_str()).parse().unwrap();
                let minor: u64 = caps.get(2).map_or("0", |m| m.as_str()).parse().unwrap();
                let patch: u64 = caps.get(3).map_or("0", |m| m.as_str()).parse().unwrap();

                let release_type = match caps.get(4).map_or("", |m| m.as_str()) {
                    "f" => Some(VersionType::Final),
                    "p" => Some(VersionType::Patch),
                    "b" => Some(VersionType::Beta),
                    "a" => Some(VersionType::Alpha),
                    _ => None,
                };

                let revision: u64 = caps.get(5).map_or("0", |m| m.as_str()).parse().unwrap();
                let base = semver::Version::new(major, minor, patch);
                Ok(Version {
                    base,
                    revision,
                    release_type: release_type.unwrap(),
                    hash: None,
                })
            }
            None => bail!("Failed to match version pattern to input"),
        }
    }
}

impl From<Installation> for Version {
    fn from(item: Installation) -> Self {
        item.version_owned()
    }
}

pub fn fetch_matching_version<I: Iterator<Item = Version>>(
    versions: I,
    version_req: semver::VersionReq,
    release_type: VersionType,
) -> io::Result<Version> {
    versions
        .filter(|version| {
            let semver_version = if version.release_type() < &release_type {
                debug!(
                    "version {} release type is smaller than specified type {:#}",
                    version, release_type
                );
                let mut semver_version = version.base().clone();
                semver_version.pre.push(semver::Identifier::AlphaNumeric(
                    version.release_type().to_string(),
                ));
                semver_version
                    .pre
                    .push(semver::Identifier::Numeric(version.revision()));
                semver_version
            } else {
                let b = version.base().clone();
                debug!(
                    "use base semver version {} of {} for comparison",
                    b, version
                );
                b
            };

            let is_match = version_req.matches(&semver_version);
            if is_match {
                info!("version {} is a match", version);
            } else {
                info!("version {} is not a match", version);
            }

            is_match
        })
        .max()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("no version found with req {}", version_req),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! invalid_version_input {
        ($($name:ident: $input:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let version_string = $input;
                    let version = Version::from_str(version_string);
                    assert!(version.is_err(), "invalid input returns None")
                }
            )*
        };
    }

    macro_rules! valid_version_input {
        ($($name:ident: $input:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let version_string = $input;
                    let version = Version::from_str(version_string);
                    assert!(version.is_ok(), "valid input returns a version")
                }
            )*
        };
    }

    invalid_version_input! {
        when_version_is_empty: "dsd",
        when_version_is_a_random_string: "sdfrersdfgsdf",
        when_version_is_a_short_version: "1.2",
        when_version_is_semver: "1.2.3",
        when_version_contains_unknown_release_type: "1.2.3g2"
    }

    valid_version_input! {
        when_version_has_single_digits: "1.2.3f4",
        when_version_has_long_digits: "0.0.0f43",
        when_version_has_only_zero_digits: "0.0.0f0"
    }

    #[test]
    fn parse_version_string_with_valid_input() {
        let version_string = "1.2.3f4";
        let version = Version::from_str(version_string);
        assert!(version.is_ok(), "valid input returns a version")
    }

    #[test]
    fn splits_version_string_into_components() {
        let version_string = "1.2.3f4";
        let version = Version::from_str(version_string).ok().unwrap();

        assert!(version.base.major == 1, "parse correct major component");
        assert!(version.base.minor == 2, "parse correct minor component");
        assert!(version.base.patch == 3, "parse correct patch component");

        assert_eq!(version.release_type, VersionType::Final);
        assert!(version.revision == 4, "parse correct revision component");
    }

    #[test]
    fn orders_version_final_release_greater_than_patch() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.2.3p4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_patch_release_greater_than_beta() {
        let version_a = Version::from_str("1.2.3p4").ok().unwrap();
        let version_b = Version::from_str("1.2.3b4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_final_release_greater_than_beta() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.2.3b4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_all_equak() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.2.3f4").ok().unwrap();
        assert_eq!(Ordering::Equal, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_major_smaller() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("0.2.3f4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_minor_smaller() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.1.3f4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_patch_smaller() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.2.2f4").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn orders_version_revision_smaller() {
        let version_a = Version::from_str("1.2.3f4").ok().unwrap();
        let version_b = Version::from_str("1.2.3f3").ok().unwrap();
        assert_eq!(Ordering::Greater, version_a.cmp(&version_b));
    }

    #[test]
    fn fetch_hash_for_known_version() {
        let version = Version::f(2017, 1, 0, 2);
        assert_eq!(version.version_hash().unwrap(), String::from("66e9e4bfc850"));
    }

    #[test]
    fn fetch_hash_for_unknown_version_yields_none() {
        let version = Version::f(2080, 2, 0, 2);
        assert!(version.version_hash().is_err());
    }

    proptest! {
        #[test]
        fn doesnt_crash(ref s in "\\PC*") {
            Version::from_str(s).is_ok();
        }

        #[test]
        fn parses_all_valid_versions(ref s in r"[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}[fpb][0-9]{1,4}") {
            Version::from_str(s).ok().unwrap();
        }

        #[test]
        fn parses_version_back_to_original(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Final,
                hash: None
            };

            let v2 = Version::from_str(&format!("{:04}.{:04}.{:04}f{:04}", major, minor, patch, revision)).ok().unwrap();
            prop_assert_eq!(v1, v2);
        }

        #[test]
        fn create_version_from_tuple(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Final,
                hash: None
            };

            let v2:Version = (major, minor, patch, revision).into();
            prop_assert_eq!(v1, v2);
        }

        #[test]
        fn create_version_final_versions(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Final,
                hash: None
            };

            let v2:Version = Version::f(major, minor, patch, revision);
            prop_assert_eq!(v1, v2);
        }

        #[test]
        fn create_version_beta_versions(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Beta,
                hash: None
            };

            let v2:Version = Version::b(major, minor, patch, revision);
            prop_assert_eq!(v1, v2);
        }

        #[test]
        fn create_version_alpha_versions(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Alpha,
                hash: None
            };

            let v2:Version = Version::a(major, minor, patch, revision);
            prop_assert_eq!(v1, v2);
        }

        #[test]
        fn create_version_patch_versions(major in 0u64..9999, minor in 0u64..9999, patch in 0u64..9999, revision in 0u64..9999 ) {
            let v1 = Version {
                base: (major,minor,patch).into(),
                revision,
                release_type: VersionType::Patch,
                hash: None
            };

            let v2:Version = Version::p(major, minor, patch, revision);
            prop_assert_eq!(v1, v2);
        }
    }
}
