use std::path::PathBuf;

use crate::config as cfg;

#[derive(Debug)]
pub struct Repo {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
    pub gitconf: PathBuf,
}

fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, String> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(format!("{} doesn't exist", path.display()))
    }
}

impl Repo {
    // new expects an existing git repo
    pub fn new(conf: cfg::Config) -> Result<Repo, String> {
        let base_path = conf.path.to_path_buf();
        let gitdir = build_path(base_path.clone(), ".git")?;
        let git_conf = build_path(gitdir.clone(), "config")?;
        Ok(Repo {
            worktree: base_path,
            gitdir: gitdir,
            gitconf: git_conf,
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
    fn repo_struct_creation_succeeds_when_in_git_repo() -> Result<(), String> {
        let worktree = utils::test_gitdir().unwrap();
        let cmd = Vec::from(["rusty-git".to_string(), "init".to_string()]);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        let _repo = Repo::new(config)?;
        Ok(())
    }
}
