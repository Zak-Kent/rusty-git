use std::num::ParseIntError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    // program errors related to git
    #[error("Not a Git repo")]
    GitNotARepo,
    #[error("Git repo already exists at the given path")]
    GitRepoAlreadyExists,
    #[error("Git malformed object content length doesn't match")]
    GitMalformedObject,
    #[error("Path doesn't exist for git object: {0}")]
    GitObjPathDoesntExist(String),
    #[error("Your current branch doesn't have any commits yet")]
    GitNoCommitsExistYet,
    #[error("ls-tree called with wrong object type: {0} is not a Tree. Check your sha.")]
    GitLsTreeWrongObjType(String),

    // program errors not related to git
    #[error("Path doesn't exist: {0}")]
    PathDoesntExist(String),
    #[error("Target dir: {0} isn't empty")]
    TargetDirNotEmpty(String),
    #[error("Target dir: {0} doesn't exist")]
    TargetDirDoesntExist(String),
    #[error("Couldn't convert dir name to utf8")]
    DirNameToUtf8Conversion,

    // wrapped errors from external libs or funcs
    #[error("IO error: {0}")]
    IOError(String),
    #[error("Error inflating a git object: {0}")]
    InflatingGitObj(String),
    #[error("Error converting bytes to utf8: {0}")]
    Utf8Conversion(#[from] std::str::Utf8Error),
    #[error("Error attempting to parse int: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Nom error: {0}")]
    NomError(String),
}

// the thiserror lib automatically does similar error
// conversion when #[from] <err_type> is used.
impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err.to_string())
    }
}

impl std::convert::From<nom::Err<nom::error::Error<&[u8]>>> for Error {
    fn from(err: nom::Err<nom::error::Error<&[u8]>>) -> Self {
        Error::NomError(err.to_string())
    }
}
