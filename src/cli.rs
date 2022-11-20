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
    },
    /// Print contents of a tree object
    LsTree {
        sha: String
    },
    /// Checkout a given sha in a given directory, the directory must be empty and created beforehand
    Checkout {
        sha: String,
        dir: String
    },
    /// Display refs available in local repo along with associated commit IDs
    ShowRef,
    /// Create or list tag objects.
    Tag {
        /// Name of the tag, if omitted command assumed to be 'rusty-git tag' which lists all tags
        name: Option<String>,
        /// Sha of the object that the tag references
        #[arg(default_value_t = String::from("HEAD"))]
        object: String,
        /// If -a flag is set a tag object will be created, if omitted only a .git/refs/tags/<name> file will be created
        #[arg(short, value_name = "Add tag object", default_value_t = false)]
        add_object: bool,
    },
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
