use std::path::PathBuf;

use crate::cli;
use crate::error as err;
use crate::objects as obj;
use crate::utils;

fn run_init(cmd: &cli::Cli) -> Result<Option<String>, err::Error> {
    let repo_path = PathBuf::from(&cmd.repo_path);
    return Ok(utils::create_git_repo(&repo_path)?);
}

fn hash_object(
    path: String,
    repo: obj::Repo,
    write_object: bool,
) -> Result<Option<String>, err::Error> {
    let path: PathBuf = PathBuf::from(path);

    let src = obj::SourceFile {
        typ: obj::GitObjTyp::Blob,
        source: path,
    };

    // by passing None to write_object it will only return the hash, no write
    let repo_arg;
    if write_object {
        repo_arg = Some(repo);
    } else {
        repo_arg = None;
    }
    return Ok(Some(obj::write_object(src, repo_arg)?));
}

// This version of cat-file differs from git's due to the fact git expects
// the object type in the args for the cmd, e.g, 'git cat-file <obj type> <sha>'
// where this version only needs the sha and then reads the obj type from
// the compressed file stored at the sha's location
fn cat_file(sha: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let file_contents = obj::read_object_as_string(&sha, repo)?;
    return Ok(Some(file_contents));
}

fn log(sha: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let head = "HEAD".to_string();
    let target_commit = match sha {
        head => utils::git_sha_from_head(&repo)?,
        _ => sha
    };

    let commit_log = utils::git_follow_commits_to_root(&target_commit, &repo)?;
    utils::git_print_commit_log(commit_log)?;
    return Ok(Some("".to_owned()));
}

pub fn run_cmd(cmd: &cli::Cli, write_object: bool) -> Result<Option<String>, err::Error> {
    let repo = obj::Repo::new(PathBuf::from(cmd.repo_path.to_owned()))?;
    let command = &cmd.command;

    match command {
        cli::GitCmd::Init => run_init(&cmd),
        cli::GitCmd::HashObject { path } => hash_object(path.to_owned(), repo, write_object),
        cli::GitCmd::CatFile { sha } => cat_file(sha.to_owned(), repo),
        cli::GitCmd::Log { sha } => log(sha.to_owned(), repo),
    }
}

#[cfg(test)]
mod object_tests {
    use std::fs::File;
    use std::io::Write;

    use super::*;
    use crate::utils;

    #[test]
    fn hash_object_returns_hash_and_cat_file_reads_test() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let cmd = cli::Cli {
            command: cli::GitCmd::HashObject {
                path: fp.to_str().unwrap().to_owned(),
            },
            repo_path: worktree.path().to_str().unwrap().to_owned(),
        };

        let hash = run_cmd(&cmd, true)?;

        assert_eq!(
            hash,
            Some("aa161e140ba95d5f611da742cedbdc98d11128a40d89a3c45b3a74f50f970897".to_owned())
        );

        let cat_cmd = cli::Cli {
            command: cli::GitCmd::CatFile { sha: hash.unwrap() },
            repo_path: worktree.path().to_str().unwrap().to_owned(),
        };

        let file_contents = run_cmd(&cat_cmd, false)?;
        assert_eq!(file_contents, Some("foobar\n".to_owned()));
        Ok(())
    }

    #[test]
    fn can_read_sha_from_head() -> Result<(), err::Error> {
        // TODO: expand this test to cover the log command when added
        let worktree = utils::test_gitdir().unwrap();
        let repo = obj::Repo::new(worktree.path().to_path_buf())?;

        utils::test_add_dummy_commit_and_update_ref_heads(&"fake-head-sha", &repo)?;

        let head_sha = utils::git_sha_from_head(&repo)?;
        assert_eq!("fake-head-sha", head_sha);
        Ok(())
    }
}
