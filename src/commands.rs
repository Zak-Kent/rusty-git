use std::path::PathBuf;

use crate::config as cfg;
use crate::error as err;
use crate::objects as obj;
use crate::utils;

fn run_init(config: &cfg::Config) -> Result<Option<String>, err::Error> {
    return Ok(utils::create_git_repo(&config.path)?);
}

fn hash_object(
    config: &cfg::Config,
    repo: Option<obj::Repo>,
) -> Result<Option<String>, err::Error> {
    match config.args.len() {
        0 => return Err(err::Error::MissingPathArgument),
        1 => (), // expects 1 path arg so do nothing
        _ => return Err(err::Error::UnrecognizedArguments(config.args.clone())),
    };
    let path: PathBuf = PathBuf::from(&config.args[0]);
    let src = obj::SourceFile {
        typ: obj::GitObjTyp::Blob,
        source: path,
    };

    // by passing None to write_object it will only return the hash, no write
    return Ok(Some(obj::write_object(src, repo)?));
}

// This version of cat-file differs from git's due to the fact git expects
// the object type in the args for the cmd, e.g, 'git cat-file <obj type> <sha>'
// where this version only needs the sha and then reads the obj type from
// the compressed file stored at the sha's location
fn cat_file(config: &cfg::Config) -> Result<Option<String>, err::Error> {
    match config.args.len() {
        0 => return Err(err::Error::MissingPathArgument),
        1 => (), // expects 1 sha arg so do nothing
        _ => return Err(err::Error::UnrecognizedArguments(config.args.clone())),
    };
    let sha = config.args[0].clone();
    let repo = obj::Repo::new(config.clone())?;
    let file_contents = obj::read_object_as_string(&sha, repo)?;
    return Ok(Some(file_contents));
}

pub fn run_cmd(config: &cfg::Config, add_repo: bool) -> Result<Option<String>, err::Error> {
    let repo;
    if add_repo {
        repo = Some(obj::Repo::new(config.clone())?);
    } else {
        repo = None;
    }

    match config.cmd {
        cfg::GitCmd::Init => run_init(&config),
        cfg::GitCmd::HashObject => hash_object(&config, repo),
        cfg::GitCmd::CatFile => cat_file(&config),
        _ => return Err(err::Error::UnimplementedCommand),
    }
}

#[cfg(test)]
mod object_tests {
    use std::fs::File;
    use std::io::Write;

    use super::*;
    use crate::utils;

    #[test]
    fn hash_object_returns_hash_and_cat_file_reads() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let cmd = utils::test_cmd("hash-object", Some(&fp.to_str().unwrap()));
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;

        let hash = run_cmd(&config, true)?;

        assert_eq!(
            hash,
            Some("aa161e140ba95d5f611da742cedbdc98d11128a40d89a3c45b3a74f50f970897".to_owned())
        );

        let cat_cmd = utils::test_cmd("cat-file", Some(&hash.unwrap()));
        let cat_config = cfg::Config::new(cat_cmd, Some(worktree.path().to_path_buf()))?;
        let file_contents = run_cmd(&cat_config, false)?;

        assert_eq!(file_contents, Some("foobar\n".to_owned()));
        Ok(())
    }

    #[test]
    fn has_object_errors_with_no_path_arg() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        // no path included in hash-object cmd
        let cmd = utils::test_cmd("hash-object", None);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;

        let missing_path = run_cmd(&config, false);
        match missing_path {
            Err(err::Error::MissingPathArgument) => assert!(true),
            _ => panic!("hash-object should error if no path arg is present"),
        };
        Ok(())
    }

    #[test]
    fn can_read_sha_from_head() -> Result<(), err::Error> {
        // TODO: expand this test to cover the log command when added
        let worktree = utils::test_gitdir().unwrap();
        let cmd = utils::test_cmd("log", None);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        let repo = obj::Repo::new(config)?;
        utils::test_add_dummy_commit_and_update_ref_heads(&"fake-head-sha", &repo)?;

        let head_sha = utils::git_sha_from_head(&repo)?;
        assert_eq!("fake-head-sha", head_sha);
        Ok(())
    }
}
