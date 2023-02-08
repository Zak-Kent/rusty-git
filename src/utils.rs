use std::fs::{read, read_dir, read_to_string};
use std::path::{Path, PathBuf};

use crate::error as err;
use crate::objects::{self as obj, tree, commit};


// ----------- git utils ---------------
pub fn is_git_repo(path: &Path) -> bool {
    let gitdir = path.join(".git");
    let conf = path.join(".git/config");
    gitdir.exists() && conf.exists()
}

pub fn git_repo_or_err(path: &Path) -> Result<PathBuf, err::Error> {
    let gitrepo = is_git_repo(path);
    if gitrepo {
        Ok(path.to_owned())
    } else {
        Err(err::Error::GitNotARepo)
    }
}

pub fn git_obj_path_from_sha(sha: &str, repo: &obj::Repo) -> Result<PathBuf, err::Error> {
    let obj_path = repo
        .gitdir
        .join(format!("objects/{}/{}", &sha[..2], &sha[2..]));

    if obj_path.exists() {
        Ok(obj_path)
    } else {
        Err(err::Error::GitObjPathDoesntExist(
            obj_path.display().to_string(),
        ))
    }
}

pub fn git_head_ref_path(repo: &obj::Repo) -> Result<PathBuf, err::Error> {
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = obj::parse_git_head(&head)?;
    Ok(repo.gitdir.join(head_ref))
}

pub fn git_sha_from_head(repo: &obj::Repo) -> Result<String, err::Error> {
    let sha_path = git_head_ref_path(repo)?;
    if sha_path.exists() {
        let sha = read_to_string(&sha_path)?.trim().to_owned();
        Ok(sha)
    } else {
        Err(err::Error::GitNoCommitsExistYet)
    }
}

pub fn git_get_tree_from_commit(
    commit: commit::Commit,
    repo: &obj::Repo,
) -> Result<tree::Tree, err::Error> {
    if let obj::GitObj::Tree(tree) = obj::read_object(&commit.tree, repo)? {
        Ok(tree)
    } else {
        Err(err::Error::GitCheckoutWrongObjType("not a tree obj".to_string()))
    }
}

pub fn git_read_index(repo: &obj::Repo) -> Result<Vec<u8>, err::Error> {
    let index_path = repo.gitdir.join("index");
    Ok(read(index_path)?)
}

pub fn git_index_exists(repo: &obj::Repo) -> bool {
    repo.gitdir.clone().join("index").exists()
}

pub fn git_check_for_rusty_git_allowed(repo: &obj::Repo) -> Result<bool, err::Error> {
    let work_path = repo.worktree.clone();
    let worktree_dir = read_dir(work_path)?;
    let mut rusty_git_allowed = false;

    for node in worktree_dir {
        let node_val = node?;
        let node_name = node_val.file_name();
        if node_name == ".rusty-git-allowed" {
            rusty_git_allowed = true;
        }
    }

    if rusty_git_allowed {
        Ok(rusty_git_allowed)
    } else {
        Err(err::Error::RustyGitAllowedFileMissing)
    }
}


pub fn get_sha_from_binary(input: &[u8]) -> String {
    let mut hexpairs = Vec::new();
    for n in input {
        hexpairs.push(format!("{:02x}", n))
    }
    hexpairs.join("")
}

// ----------- fs utils ---------------
pub fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        Ok(path)
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

#[cfg(test)]
mod utils_tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn return_true_when_git_repo() {
        // need the two var decs below to get around the borrow checker
        // not seeing that the ref .path() creates should be bound
        // through the unwrap when written: Result<TempDir>.unwrap().path()
        let gitdir = test_utils::test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path))
    }

    #[test]
    fn return_false_when_not_git_repo() {
        let tempdir = test_utils::test_tempdir().unwrap();
        let tempdir_path = tempdir.path();
        assert!(!is_git_repo(tempdir_path))
    }

    #[test]
    fn dir_is_empty_works_as_expected() {
        let tempdir = test_utils::test_tempdir().unwrap();
        let gitdir = test_utils::test_gitdir().unwrap();
        assert_eq!(Ok(true), test_utils::dir_is_empty(tempdir.path()));
        assert_eq!(Ok(false), test_utils::dir_is_empty(gitdir.path()));
    }
}
