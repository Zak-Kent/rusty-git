use std::path::PathBuf;
use crate::utils;

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
    fn new(cmd: &str) -> Result<GitCmd, &'static str> {
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
            _ => Err("Command isn't supported"),
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
    pub fn new(cmds: Vec<String>, repo_path: Option<PathBuf>) -> Result<Config, &'static str> {
        if cmds.len() == 1 {
            Err("No command entered")
        } else {
            let gcmd = GitCmd::new(&cmds[1])?;
            let repo_path = repo_path.unwrap_or(PathBuf::from("."));

            Ok(Config {
                cmd: gcmd,
                path: utils::git_repo_or_err(repo_path.as_path())?,
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
    fn config_struct_creation_fails_when_not_in_git_repo() -> Result<(), String> {
        let tmpdir = utils::test_tempdir().unwrap();
        let cmd = Vec::from(["rusty-git".to_string(), "add".to_string()]);
        let config = Config::new(cmd, Some(tmpdir.path().to_path_buf()));
        assert!(config.is_err());
        match config {
            Err(e) => assert!(
                e.contains("Not a git repository!"),
                "missing expected git repo error"
            ),
            _ => panic!("Config creation should error!"),
        };
        Ok(())
    }
}
