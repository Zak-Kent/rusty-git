use std::fs;
use std::path::PathBuf;

use crate::error as err;
use crate::utils;

#[derive(Debug, Clone)]
pub struct Repo {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
    pub gitconf: String,
}

impl Repo {
    // new expects an existing git repo
    pub fn new(path: PathBuf) -> Result<Repo, err::Error> {
        let base_path = utils::git_repo_or_err(&PathBuf::from(path))?;
        let gitdir = utils::build_path(base_path.clone(), ".git")?;
        let gitconf_path = utils::build_path(gitdir.clone(), "config")?;
        let gitconf = fs::read_to_string(gitconf_path)?;

        Ok(Repo {
            worktree: base_path,
            gitdir,
            gitconf,
        })
    }
}

#[allow(dead_code)]
pub fn find_gitdir_and_create_repo(path: String) -> Result<Repo, err::Error> {
    let mut path = PathBuf::from(path);

    while !utils::is_git_repo(&path) {
        if let Some(p) = path.parent() {
            path = p.to_path_buf();
        } else {
            return Err(err::Error::GitNotARepo);
        }
    }

    return Ok(Repo::new(path)?);
}

#[cfg(test)]
mod object_tests {
    use std::fs::{create_dir_all, File};

    use super::*;
    use crate::test_utils;
    use crate::utils;
    use crate::object_mods as objm;

    #[test]
    fn git_repo_setup_test() {
        // unwrap will panic here if dir setup fails
        let worktree = test_utils::test_gitdir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join(".git/config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }

    #[test]
    fn repo_struct_creation_succeeds_when_in_git_repo() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();
        let _repo = Repo::new(worktree.path().to_path_buf())?;
        Ok(())
    }

    #[test]
    fn repo_struct_creation_fails_when_not_in_git_repo() -> Result<(), err::Error> {
        let tmpdir = test_utils::test_tempdir().unwrap();
        let repo = Repo::new(tmpdir.path().to_path_buf());
        assert!(repo.is_err());
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_finds_parent_gitdir() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();

        // create a nested path with .git living a few levels above
        let nested_path = worktree.path().join("foo/bar/baz");
        create_dir_all(&nested_path)?;

        let repo = find_gitdir_and_create_repo(nested_path.to_str().unwrap().to_owned())?;

        // check nested path was discarded when creating Repo.worktree
        assert_eq!(worktree.path(), repo.worktree);
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_errors_when_no_gitdir_in_path() -> Result<(), err::Error> {
        let tmpdir = test_utils::test_tempdir().unwrap();

        let repo = find_gitdir_and_create_repo(tmpdir.path().to_str().unwrap().to_owned());
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

}
