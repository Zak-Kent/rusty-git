use std::fs::{create_dir, File};
use std::io::Error;
use tempfile::{tempdir, TempDir};

pub fn test_tempdir() -> Result<TempDir, Error> {
    let tmp_dir = tempdir()?;
    Ok(tmp_dir)
}

#[allow(dead_code)]
pub fn test_git_dir() -> Result<TempDir, Error> {
    let dir = test_tempdir()?;
    create_dir(dir.path().join(".git"))?;
    File::create(dir.path().join(".git/config"))?;
    return Ok(dir);
}
