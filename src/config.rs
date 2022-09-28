use std::path::PathBuf;

use crate::error as err;

#[derive(Debug)]
pub enum GitCmd {
    Add,
    CatFile,
    Checkout,
    Commit,
    HashObject,
    Init,
    Log,
    LsTree,
    Merge,
    Rebase,
    RevParse,
    Rm,
    ShowRef,
    Tag,
}

impl GitCmd {
    fn new(cmd: &str) -> Result<GitCmd, err::Error> {
        match cmd {
            "add" => Ok(GitCmd::Add),
            "cat-file" => Ok(GitCmd::CatFile),
            "checkout" => Ok(GitCmd::Checkout),
            "commit" => Ok(GitCmd::Commit),
            "hash-object" => Ok(GitCmd::HashObject),
            "init" => Ok(GitCmd::Init),
            "log" => Ok(GitCmd::Log),
            "ls-tree" => Ok(GitCmd::LsTree),
            "merge" => Ok(GitCmd::Merge),
            "rebase" => Ok(GitCmd::Rebase),
            "rev-parse" => Ok(GitCmd::RevParse),
            "rm" => Ok(GitCmd::Rm),
            "show-ref" => Ok(GitCmd::ShowRef),
            "tag" => Ok(GitCmd::Tag),
            _ => Err(err::Error::UnsupportedCommand),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub cmd: GitCmd,
    pub path: PathBuf,
    pub args: Vec<String>,
}

impl Config {
    pub fn new(cmds: Vec<String>, repo_path: Option<PathBuf>) -> Result<Config, err::Error> {
        if cmds.len() == 1 {
            Err(err::Error::MissingCommand)
        } else {
            let gcmd = GitCmd::new(&cmds[1])?;
            let repo_path = repo_path.unwrap_or(PathBuf::from("."));

            Ok(Config {
                cmd: gcmd,
                path: repo_path,
                args: cmds[2..].to_vec(),
            })
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use crate::utils;

    #[test]
    fn config_creation_fails_on_unsupported_command() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();
        let cmd = utils::test_cmd("foo");
        let config = Config::new(cmd, Some(worktree.path().to_path_buf()));
        assert!(config.is_err());
        match config {
            Err(err::Error::UnsupportedCommand) => assert!(true),
            _ => panic!("Config creation should error on unsupported foo command!"),
        };
        Ok(())
    }

    #[test]
    fn config_creation_succeeds_on_supported_command() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();
        let cmd = utils::test_cmd("add");
        let _config = Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        Ok(())
    }
}
