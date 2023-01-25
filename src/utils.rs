use std::fs::{metadata, read, read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::str::from_utf8;

use crate::error as err;
use crate::objects as obj;
use crate::object_mods::{self as objm, tree, commit};


// ----------- git utils ---------------
pub fn is_git_repo(path: &Path) -> bool {
    let gitdir = path.join(".git");
    let conf = path.join(".git/config");
    gitdir.exists() && conf.exists()
}

pub fn git_repo_or_err(path: &Path) -> Result<PathBuf, err::Error> {
    let gitrepo = is_git_repo(path);
    if gitrepo {
        return Ok(path.to_owned());
    } else {
        Err(err::Error::GitNotARepo)
    }
}

pub fn git_obj_path_from_sha(sha: &str, repo: &obj::Repo) -> Result<PathBuf, err::Error> {
    let obj_path = repo
        .gitdir
        .join(format!("objects/{}/{}", &sha[..2], &sha[2..]));

    if obj_path.exists() {
        return Ok(obj_path.to_path_buf());
    } else {
        return Err(err::Error::GitObjPathDoesntExist(
            obj_path.display().to_string(),
        ));
    }
}

pub fn git_sha_from_head(repo: &obj::Repo) -> Result<String, err::Error> {
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = objm::parse_git_head(&head)?;
    let sha_path = repo.gitdir.join(head_ref);

    if sha_path.exists() {
        let sha = read_to_string(&sha_path)?.trim().to_owned();
        return Ok(sha);
    } else {
        return Err(err::Error::GitNoCommitsExistYet);
    };
}

pub fn git_get_tree_from_commit(
    sha: &str,
    contents: &[u8],
    repo: &obj::Repo,
) -> Result<tree::Tree, err::Error> {
    let commit::KvsMsg { kvs, .. } = commit::parse_kv_list_msg(contents, sha)?;

    let tree_sha = match kvs.get("tree".as_bytes()) {
        Some(s) => from_utf8(s)?,
        None => return Err(err::Error::GitNoTreeKeyInCommit),
    };

    let obj::GitObject { contents, .. } = obj::read_object(tree_sha, repo)?;
    let tree = tree::parse_git_tree(&contents)?;

    return Ok(tree);
}

pub fn git_read_index(repo: &obj::Repo) -> Result<Vec<u8>, err::Error> {
    let index_path = repo.gitdir.join("index");
    return Ok(read(index_path)?);
}

pub fn git_index_exists(repo: &obj::Repo) -> bool {
    return repo.gitdir.clone().join("index").exists();
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
        return Ok(rusty_git_allowed);
    } else {
        return Err(err::Error::RustyGitAllowedFileMissing);
    }
}


pub fn get_sha_from_binary(input: &[u8]) -> String {
    let mut hexpairs = Vec::new();
    for n in input {
        hexpairs.push(format!("{:02x}", n))
    }
    return hexpairs.join("");
}

// ----------- fs utils ---------------
pub fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

pub fn content_length(path: &Path) -> Result<u64, err::Error> {
    Ok(metadata(path)?.len())
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
