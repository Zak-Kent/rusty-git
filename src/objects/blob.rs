use std::fmt;
use std::fs::read;
use std::path::PathBuf;
use std::str::from_utf8;

use super::{AsBytes, GitObj};
use crate::error as err;

#[derive(Debug, PartialEq)]
pub struct Blob {
    pub contents: Vec<u8>,
    pub len: usize,
}

impl Blob {
    pub fn new(contents: &[u8]) -> Blob {
        Blob {
            len: contents.len(),
            contents: contents.to_vec(),
        }
    }
}

pub fn blob_from_path(path: PathBuf) -> Result<GitObj, err::Error> {
    let blob_contents = read(path)?;
    return Ok(GitObj::Blob(Blob::new(&blob_contents)));
}

impl AsBytes for Blob {
    fn as_bytes(self: &Blob) -> Vec<u8> {
        [
            "blob".as_bytes(),
            " ".as_bytes(),
            self.len.to_string().as_bytes(),
            "\x00".as_bytes(),
            self.contents.as_slice(),
        ]
        .concat()
        .to_vec()
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = from_utf8(&self.contents);
        if let Err(utf8_conversion_err) = output {
            println!("Error converting blob to utf8: {}", utf8_conversion_err);
            return Err(fmt::Error);
        } else {
            write!(f, "{}", output.unwrap())
        }
    }
}
