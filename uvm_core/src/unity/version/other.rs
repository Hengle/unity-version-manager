use super::*;
use crate::error::Result;
use crate::error::UvmErrorKind;
use std::convert::AsRef;
use std::path::Path;

pub fn read_version_from_path<P: AsRef<Path>>(path: P) -> Result<Version> {
    Err(UvmErrorKind::IllegalOperation("fn 'read_version_from_path' not supported on current platform".to_string()).into())
}
