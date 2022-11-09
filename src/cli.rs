use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
pub enum GitCmd {
    /// Create an empty git repo, errors if git repo already exists
    Init,
    /// Returns the sha256 hash of the file at the given path
    HashObject {
        path: String,
    },
    /// Print the contents of the .git/objects file at the given sha
    CatFile {
        sha: String,
    },
    /// Print commits starting at the given sha, defaults to HEAD
    Log {
        #[arg(default_value_t = String::from("HEAD"))]
        sha: String,
    }
}

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub command: GitCmd,
    /// Sets the path of the repo where git command will be executed
    #[arg(default_value_t = String::from("."))]
    pub repo_path: String,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
