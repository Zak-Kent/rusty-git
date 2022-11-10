use std::fs::{create_dir_all, metadata, read, read_to_string, File};
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use tempfile::{tempdir, TempDir};

use crate::error as err;
use crate::object_parsers as objp;
use crate::objects as obj;

// ----------- test utils ---------------
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
pub fn test_cmd(cmd: &str, arg: Option<&str>) -> Vec<String> {
    let mut cmd_result = Vec::from(["rusty-git".to_owned(), cmd.to_owned()]);

    if let Some(arg) = arg {
        cmd_result.push(arg.to_owned());
    }

    return cmd_result;
}

#[allow(dead_code)]
pub fn test_add_dummy_commit_and_update_ref_heads(
    sha: &str,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    //TODO: expand this to add an actual commit in .git/objects later
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = objp::parse_git_head(&head)?;
    let mut ref_file = File::create(repo.gitdir.join(head_ref))?;
    writeln!(ref_file, "{}", sha)?;
    Ok(())
}

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

pub fn default_repo_config() -> &'static str {
    "[core]
       bare = false
       filemode = false
       repositoryformatversion = 0"
}

pub fn create_git_repo(path: &Path) -> Result<Option<String>, err::Error> {
    if is_git_repo(path) {
        return Err(err::Error::GitRepoAlreadyExists);
    }
    create_dir_all(path.join(".git/objects"))?;
    create_dir_all(path.join(".git/refs/heads"))?;
    create_dir_all(path.join(".git/refs/tags"))?;

    let mut head = File::create(path.join(".git/HEAD"))?;
    writeln!(head, "ref: refs/heads/master")?;

    let mut description = File::create(path.join(".git/description"))?;
    writeln!(
        description,
        "Unnamed repository; edit this file 'description' to name the repository."
    )?;

    let mut config = File::create(path.join(".git/config"))?;
    writeln!(config, "{}", default_repo_config())?;
    Ok(None)
}

pub fn git_obj_path_from_sha(sha: &str, repo: obj::Repo) -> Result<PathBuf, err::Error> {
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
    let head_ref = objp::parse_git_head(&head)?;
    let sha_path = repo.gitdir.join(head_ref);

    if sha_path.exists() {
        let sha = read_to_string(&sha_path)?.trim().to_owned();
        return Ok(sha);
    } else {
        return Err(err::Error::GitNoCommitsExistYet);
    };
}

pub fn git_read_commit(sha: &str, repo: &obj::Repo) -> Result<objp::KvsMsg, err::Error> {
    let commit = obj::read_object(sha, repo.clone())?;
    let parsed_commit = objp::parse_kv_list_msg(&commit.contents, sha)?;
    return Ok(parsed_commit);
}

pub fn git_follow_commits_to_root(
    sha: &str,
    repo: &obj::Repo,
) -> Result<Vec<objp::KvsMsg>, err::Error> {
    let mut commit = git_read_commit(sha, &repo)?;
    let mut commit_log: Vec<objp::KvsMsg> = Vec::new();

    // add the first commit to log
    commit_log.push(commit.clone());

    while let Some(parent) = &commit.kvs.get("parent".as_bytes()) {
        let parent_sha = from_utf8(parent)?;
        commit = git_read_commit(parent_sha, &repo)?;

        // add parent commits to log
        commit_log.push(commit.clone());
    }

    // add root commit to log
    commit_log.push(commit.clone());

    return Ok(commit_log);
}

pub fn git_commit_to_string(commit: &objp::KvsMsg) -> Result<String, err::Error> {
    let mut output = String::new();
    output.push_str(&format!("commit: {}\n", commit.sha));
    output.push_str(&format!(
        "Author: {}\n",
        from_utf8(commit.kvs.get("author".as_bytes()).unwrap())?
    ));
    output.push_str(&format!("\n{}\n", from_utf8(&commit.msg)?));
    return Ok(output);
}

pub fn git_commit_log_to_string(commit_log: Vec<objp::KvsMsg>) -> Result<String, err::Error> {
    let mut output = String::new();
    for commit in commit_log {
        output.push_str(&git_commit_to_string(&commit)?);
    }
    return Ok(output);
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
    use ini;
    use std::collections::HashMap;

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
                .unwrap_or_else(|err| panic!(".git/{} should exist. Error: {}", ext, err)))
        };
        assert_path("objects");
        assert_path("refs/tags");
        assert_path("refs/heads");
        assert_path("HEAD");
        assert_path("config");
        assert_path("description");

        let mut core: HashMap<String, Option<String>> = HashMap::new();
        core.insert("filemode".to_owned(), Some("false".to_owned()));
        core.insert("repositoryformatversion".to_owned(), Some("0".to_owned()));
        core.insert("bare".to_owned(), Some("false".to_owned()));

        let mut expected_config: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();
        expected_config.insert("core".to_owned(), core);

        let config = ini::ini!(gitdir_path.join("config").to_str().unwrap());
        assert_eq!(expected_config, config);
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
