use std::{num::ParseIntError, path::StripPrefixError, fmt::Debug};

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
    #[error("checkout called with wrong object type: {0} is not a Tree or Commit. Check your sha.")]
    GitCheckoutWrongObjType(String),
    #[error("Git commit doesn't have a tree entry")]
    GitNoTreeKeyInCommit,
    #[error("Git tree contains object other than blob or tree")]
    GitTreeInvalidObject,
    #[error("Git tag -a isn't implemented yet")]
    GitCreateTagObjectNotImplemented,
    #[error("Unrecognized git index version: {0}, this tool only supports version 2")]
    GitUnrecognizedIndexVersion(u32),

    // program errors not related to git
    #[error("Path doesn't exist: {0}")]
    PathDoesntExist(String),
    #[error("Target dir: {0} isn't empty")]
    TargetDirNotEmpty(String),
    #[error("Target dir: {0} doesn't exist")]
    TargetDirDoesntExist(String),
    #[error("Couldn't convert dir name to utf8")]
    DirNameToUtf8Conversion,
    #[error("Path wasn't valid utf8")]
    PathToUtf8Conversion,
    #[error("Timestamp conversion error")]
    TimestampConversionError,

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
    #[error("StripPrefixError: {0}")]
    StripPrefixError(#[from] StripPrefixError),
}

// the thiserror lib automatically does similar error
// conversion when #[from] <err_type> is used.
impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err.to_string())
    }
}

impl<T: Debug> std::convert::From<nom::Err<nom::error::Error<T>>> for Error {
    fn from(err: nom::Err<nom::error::Error<T>>) -> Self {
        Error::NomError(format!("{:?}", err))
    }
}
