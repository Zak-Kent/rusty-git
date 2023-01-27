use std::str::from_utf8;

use crate::error as err;
use crate::object_mods::{self as objm, commit};
use crate::objects as obj;

pub fn read_commit(sha: &str, repo: &obj::Repo) -> Result<commit::Commit, err::Error> {
    if let objm::GitObj::Commit(commit) = objm::read_object(sha, repo)? {
        return Ok(commit);
    } else {
        return Err(err::Error::GitUnexpectedInternalType(format!(
            "{:?}",
            "Expected a commit object"
        )));
    }
}

pub fn commit_to_string(commit: &commit::Commit) -> Result<String, err::Error> {
    let mut output = String::new();
    output.push_str(&format!("commit: {}\n", commit.sha));
    output.push_str(&format!(
        "Author: {}\n",
        from_utf8(commit.kvs.get("author".as_bytes()).unwrap())?
    ));
    output.push_str(&format!("\n{}\n", from_utf8(&commit.msg)?));
    return Ok(output);
}

pub fn follow_commits_to_root(
    sha: &str,
    repo: &obj::Repo,
) -> Result<Vec<commit::Commit>, err::Error> {
    let mut commit = read_commit(sha, &repo)?;
    let mut commit_log: Vec<commit::Commit> = Vec::new();

    // add the first commit to log
    commit_log.push(commit.clone());

    while let Some(parent) = &commit.kvs.get("parent".as_bytes()) {
        let parent_sha = from_utf8(parent)?;
        commit = read_commit(parent_sha, &repo)?;

        // add parent commits to log
        commit_log.push(commit.clone());
    }
    return Ok(commit_log);
}

pub fn commit_log_to_string(commit_log: Vec<commit::Commit>) -> Result<String, err::Error> {
    let mut output = String::new();
    for commit in commit_log {
        output.push_str(&commit_to_string(&commit)?);
    }
    return Ok(output);
}
