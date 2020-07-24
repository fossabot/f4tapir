use std::io::{Error, ErrorKind, Result};
use std::path::Path;

pub fn path_as_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Non-UTF-8 filenames not supported"))
}
