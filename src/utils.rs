use std::fs::{create_dir, File};
use std::io::Error;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

use crate::error as err;

pub fn test_tempdir() -> Result<TempDir, Error> {
    let tmp_dir = tempdir()?;
    Ok(tmp_dir)
}

#[allow(dead_code)]
pub fn test_gitdir() -> Result<TempDir, Error> {
    let dir = test_tempdir()?;
    create_dir(dir.path().join(".git"))?;
    File::create(dir.path().join(".git/config"))?;
    return Ok(dir);
}

#[allow(dead_code)]
pub fn test_cmd(cmd: &str) -> Vec<String>{
    return Vec::from(["rusty-git".to_owned(), cmd.to_owned()]);
}

pub fn is_git_repo(path: &Path) -> bool {
    let gitdir = path.join(".git");
    let conf = path.join(".git/config");
    gitdir.exists() && conf.exists()
}

pub fn git_repo_or_err(path: &Path) -> Result<PathBuf, err::Error> {
    let gitrepo = is_git_repo(path);
    if gitrepo {
        return Ok(path.to_owned())
    } else {
        Err(err::Error::NotAGitRepo)
    }
}

#[cfg(test)]
mod utils_tests {
    use super::*;

    #[test]
    fn return_true_when_git_repo() {
        // need the two var decs below to get around the borrow checker
        // not seeing that the ref .path() creates should be bound
        // through the unwrap when written: Result<TempDir>.unwrap().path()
        let gitdir = test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path))
    }

    #[test]
    fn return_false_when_not_git_repo() {
        let tempdir = test_tempdir().unwrap();
        let tempdir_path = tempdir.path();
        assert!(!is_git_repo(tempdir_path))
    }
}
