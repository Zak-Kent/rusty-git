use crate::error as err;
use crate::objects::{self as obj, commit};

pub fn read_commit(sha: &str, repo: &obj::Repo) -> Result<commit::Commit, err::Error> {
    if let obj::GitObj::Commit(commit) = obj::read_object(sha, repo)? {
        Ok(commit)
    } else {
        Err(err::Error::GitUnexpectedInternalType(format!(
            "{:?}",
            "Expected a commit object"
        )))
    }
}

pub fn commit_to_string(commit: &commit::Commit) -> Result<String, err::Error> {
    let mut output = String::new();
    output.push_str(&format!("commit: {}\n", commit.sha));
    output.push_str(&format!("Author: {}\n", commit.author));
    output.push_str(&format!("\n{}\n", commit.msg));
    Ok(output)
}

pub fn follow_commits_to_root(
    sha: &str,
    repo: &obj::Repo,
) -> Result<Vec<commit::Commit>, err::Error> {
    let mut commit = read_commit(sha, repo)?;
    let mut commit_log: Vec<commit::Commit> = Vec::new();

    // add the first commit to log
    commit_log.push(commit.clone());

    while let Some(parent) = commit.parent {
        commit = read_commit(&parent, repo)?;
        commit_log.push(commit.clone()); // add parent commits to log
    }
    Ok(commit_log)
}

pub fn commit_log_to_string(commit_log: Vec<commit::Commit>) -> Result<String, err::Error> {
    let mut output = String::new();
    for commit in commit_log {
        output.push_str(&commit_to_string(&commit)?);
    }
    Ok(output)
}
