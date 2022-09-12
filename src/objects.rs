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
    pub fn new(cmd: cfg::Config) -> Result<Repo, String> {
        let base_path = cmd.path.to_path_buf();
        let git_dir = build_path(base_path.clone(), ".git")?;
        let git_conf = build_path(git_dir.clone(), "config")?;
        Ok(Repo {
            worktree: base_path,
            gitdir: git_dir,
            gitconf: git_conf,
        })
    }
}

#[cfg(test)]
mod object_tests {
    use crate::testutils as utils;

    #[test]
    fn git_repo_setup_test() {
        // unwrap will panic here if dir setup fails
        let worktree = utils::test_git_dir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join(".git/config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }
}
