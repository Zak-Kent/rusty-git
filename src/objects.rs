use std::fs;
use std::path::{PathBuf, Path};
use sha256;

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

pub fn find_gitdir_and_create_repo(conf: cfg::Config) -> Result<Repo, err::Error> {
    let mut path = conf.path;
    while !utils::is_git_repo(&path) {
        if let Some(p) = path.parent() {
            path = p.to_path_buf();
        } else {
            return Err(err::Error::NotAGitRepo);
        }
    }

    let updated_conf = cfg::Config { path, ..conf };
    return Ok(Repo::new(updated_conf)?);
}

pub fn hash_object(path: &Path) -> Result<String, err::Error> {
    let hash = sha256::digest_file(path)?;
    return Ok(hash.to_owned());
}

#[cfg(test)]
mod object_tests {
    use std::fs::{create_dir_all, File};
    use std::io::Write;

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

    #[test]
    fn find_gitdir_and_create_repo_finds_parent_gitdir() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        // create a nested path with .git living a few levels above
        let nested_path = worktree.path().join("foo/bar/baz");
        create_dir_all(&nested_path)?;

        let cmd = utils::test_cmd("add");
        let config = cfg::Config::new(cmd, Some(nested_path))?;
        let repo = find_gitdir_and_create_repo(config)?;

        // check nested path was discarded when creating Repo.worktree
        assert_eq!(worktree.path(), repo.worktree);
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_errors_when_no_gitdir_in_path() -> Result<(), err::Error> {
        let tmpdir = utils::test_tempdir().unwrap();
        let cmd = utils::test_cmd("add");
        let config = cfg::Config::new(cmd, Some(tmpdir.path().to_path_buf()))?;

        let repo = find_gitdir_and_create_repo(config);
        match repo {
            Err(err::Error::NotAGitRepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

    #[test]
    fn generate_hash_for_file() -> Result<(), err::Error> {
        let tmpdir = utils::test_tempdir().unwrap();
        let fp = tmpdir.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;
        let hash = hash_object(&fp)?;

        assert_eq!(hash,
                   "aec070645fe53ee3b3763059376134f058cc337247c978add178b6ccdfb0019f".to_owned());
        Ok(())
    }
}
