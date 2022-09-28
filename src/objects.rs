use std::path::PathBuf;
use std::fs;

use crate::config as cfg;
use crate::error as err;
use crate::utils;

#[derive(Debug)]
pub struct Repo {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
    pub gitconf: String,
}

fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

impl Repo {
    // new expects an existing git repo
    pub fn new(conf: cfg::Config) -> Result<Repo, err::Error> {
        let base_path = utils::git_repo_or_err(&conf.path)?;
        let gitdir = build_path(base_path.clone(), ".git")?;
        let gitconf_path = build_path(gitdir.clone(), "config")?;
        let gitconf = fs::read_to_string(gitconf_path)?;

        Ok(Repo {
            worktree: base_path,
            gitdir,
            gitconf,
        })
    }
}

#[cfg(test)]
mod object_tests {
    use super::*;
    use crate::utils;

    #[test]
    fn git_repo_setup_test() {
        // unwrap will panic here if dir setup fails
        let worktree = utils::test_gitdir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join(".git/config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }

    #[test]
    fn repo_struct_creation_succeeds_when_in_git_repo() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();
        let cmd = utils::test_cmd("init");
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        let _repo = Repo::new(config)?;
        Ok(())
    }

    #[test]
    fn repo_struct_creation_fails_when_not_in_git_repo() -> Result<(), err::Error> {
        let tmpdir = utils::test_tempdir().unwrap();
        let cmd = utils::test_cmd("add");
        let config = cfg::Config::new(cmd, Some(tmpdir.path().to_path_buf()))?;
        let repo = Repo::new(config);
        assert!(repo.is_err());
        match repo {
            Err(err::Error::NotAGitRepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }
}
