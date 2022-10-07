use std::num::ParseIntError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Not a Git repo")]
    NotAGitRepo,
    #[error("Git repo already exists at the given path")]
    GitRepoAlreadyExists,
    #[error("Git malformed object content length doesn't match")]
    GitMalformedObject,
    #[error("Missing a command argument")]
    MissingCommand,
    #[error("Unsupported command")]
    UnsupportedCommand,
    #[error("Unimplemented command")]
    UnimplementedCommand,
    #[error("Path doesn't exist: {0}")]
    PathDoesntExist(String),
    #[error("Path doesn't exist for git object: {0}")]
    GitObjPathDoesntExist(String),
    #[error("IO error: {0}")]
    IOError(String),
    #[error("Unrecognized arguments passed in with command: {0:?}")]
    UnrecognizedArguments(Vec<String>),
    #[error("Command expects a path as an argument")]
    MissingPathArgument,
    #[error("Error inflating a git object: {0}")]
    InflatingGitObj(String),
    #[error("Error converting bytes to utf8: {0}")]
    Utf8Conversion(#[from] std::str::Utf8Error),
    #[error("Error attempting to parse this int: {0}")]
    ParseIntError(#[from] ParseIntError),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err.to_string())
    }
}
