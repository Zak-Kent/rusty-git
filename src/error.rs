#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Not a Git repo")]
    NotAGitRepo,
    #[error("Git repo already exists at the given path")]
    GitRepoAlreadyExists,
    #[error("Missing a command argument")]
    MissingCommand,
    #[error("Unsupported command")]
    UnsupportedCommand,
    #[error("Unimplemented command")]
    UnimplementedCommand,
    #[error("Path doesn't exist: {0}")]
    PathDoesntExist(String),
    #[error("IO error: {0}")]
    IOError(String),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err.to_string())
    }
}
