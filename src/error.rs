#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Not a Git repo")]
    NotAGitRepo,
    #[error("Missing a command argument")]
    MissingCommand,
    #[error("Unsupported command")]
    UnsupportedCommand,
    #[error("Path doesn't exist: {0}")]
    PathDoesntExist(String),
}
