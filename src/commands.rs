use std::path::PathBuf;

use crate::config as cfg;
use crate::error as err;
use crate::objects as obj;
use crate::utils;

fn run_init(config: &cfg::Config) -> Result<Option<String>, err::Error> {
    return Ok(utils::create_git_repo(&config.path)?);
}

fn hash_object(config: &cfg::Config) -> Result<Option<String>, err::Error> {
    match config.args.len() {
        0 => return Err(err::Error::MissingPathArgument),
        1 => (), // expects 1 path arg so do nothing
        _ => return Err(err::Error::UnrecognizedArguments(config.args.clone())),
    };
    let path: PathBuf = PathBuf::from(&config.args[0]);
    let blob = obj::GitObject::Blob(path);

    // by passing None to write_object it will only return the hash, no write
    return Ok(Some(obj::write_object(blob, None)?));
}

pub fn run_cmd(config: &cfg::Config) -> Result<Option<String>, err::Error> {
    match config.cmd {
        cfg::GitCmd::Init => run_init(&config),
        cfg::GitCmd::HashObject => hash_object(config),
        _ => return Err(err::Error::UnimplementedCommand),
    }
}

#[cfg(test)]
mod object_tests {
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    use super::*;
    use crate::utils;

    #[test]
    fn hash_object_returns_hash() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let cmd = utils::test_cmd("hash-object", Some(&fp.to_str().unwrap()));
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;

        let hash = run_cmd(&config)?;

        assert_eq!(
            hash,
            Some("aa161e140ba95d5f611da742cedbdc98d11128a40d89a3c45b3a74f50f970897".to_owned())
        );
        Ok(())
    }

    #[test]
    fn has_object_errors_with_no_path_arg() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        // no path included in hash-object cmd
        let cmd = utils::test_cmd("hash-object", None);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;

        let missing_path = run_cmd(&config);
        match missing_path {
            Err(err::Error::MissingPathArgument) => assert!(true),
            _ => panic!("hash-object should error if no path arg is present"),
        };
        Ok(())
    }
}
