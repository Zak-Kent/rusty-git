use std::path::Path;

use crate::config as cfg;

fn does_git_dir_exist(path: Box<&Path>) -> bool {
    let git_dir = format!("{}git", path.display());
    Path::new(&git_dir).exists()
}

pub struct Repo {
    worktree: String,
    gitdir: String,
    gitconf: String,
}

impl Repo {
    pub fn new(cmd: cfg::Config) {
        if  does_git_dir_exist(cmd.path) {
            println!(".git dir exists");
        } else {
            println!("need to create .git dir");
        }

    }
}
