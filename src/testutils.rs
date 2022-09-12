use tempfile::{tempdir, TempDir};
use std::fs::{create_dir, File};
use std::io::Error;

#[allow(dead_code)]
pub fn test_git_dir () -> Result<TempDir, Error> {
    let dir = tempdir()?;
    create_dir(dir.path().join(".git"))?;
    File::create(dir.path().join(".git/config"))?;
    return Ok(dir)
}
