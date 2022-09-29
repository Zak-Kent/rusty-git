use std::fs::{File, create_dir_all};
use std::io::Error;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

use crate::error as err;

pub fn test_tempdir() -> Result<TempDir, Error> {
    let tmp_dir = tempdir()?;
    Ok(tmp_dir)
}

#[allow(dead_code)]
pub fn test_gitdir() -> Result<TempDir, err::Error> {
    let dir = test_tempdir()?;
    create_git_repo(dir.path())?;
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

pub fn create_git_repo(path: &Path) -> Result<(), err::Error> {
    if is_git_repo(path) {
        return Err(err::Error::GitRepoAlreadyExists)
    }
    create_dir_all(path.join(".git/objects"))?;
    create_dir_all(path.join(".git/refs/heads"))?;
    create_dir_all(path.join(".git/refs/tags"))?;
    File::create(path.join(".git/HEAD"))?;
    File::create(path.join(".git/config"))?;
    File::create(path.join(".git/description"))?;
    Ok(())
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

    #[test]
    fn create_gitdir_succeeds_in_empty_dir() {
        let tempdir = test_tempdir().unwrap();
        let tempdir_path = tempdir.path();

        let create_git_repo_result = create_git_repo(&tempdir_path);
        if create_git_repo_result.is_err() {
            panic!("repo setup failed in test!")
        }

        let gitdir_path = tempdir_path.join(".git");
        assert!(gitdir_path.try_exists().expect(".git dir should exist"));

        let assert_path = |ext: &str| {
            assert!(gitdir_path
                    .join(ext)
                    .try_exists()
                    .unwrap_or_else(|err|
                                    panic!(".git/{} should exist. Error: {}", ext, err)))
        };
        assert_path("objects");
        assert_path("refs/tags");
        assert_path("refs/heads");
        assert_path("HEAD");
        assert_path("config");
        assert_path("description");
    }

    #[test]
    fn create_gitdir_errors_in_an_existing_git_dir() {
        let gitdir = test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path));

        let create_git_repo_result = create_git_repo(gitdir_path);
        assert!(Err(err::Error::GitRepoAlreadyExists) == create_git_repo_result);
    }
}
