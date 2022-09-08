use std::fs::{self, ReadDir};
use std::path::PathBuf;

use crate::config as cfg;

fn does_git_dir_exist(dir: ReadDir) -> bool {
    dir.into_iter()
        .any(|f| f.unwrap().path() == PathBuf::from("./.git"))
}

pub struct Repo {
    worktree: String,
    gitdir: String,
    gitconf: String,
}

impl Repo {
    pub fn new(cmd: cfg::Config) {
        // TODO: need to pass in the path as part of the config
        println!("file path we're searching on: {:?}", *cmd.path);

        let dir_contents = fs::read_dir(*cmd.path);

        if does_git_dir_exist(dir_contents.unwrap()) {
            println!(".git dir exists");
        } else {
            println!("need to create .git dir");
        }
    }
}
