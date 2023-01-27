use super::AsBytes;
use std::fmt;
use std::str::from_utf8;

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

impl AsBytes for Blob {
    fn as_bytes(self: &Blob) -> Vec<u8> {
        self.contents.clone()
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = from_utf8(&self.contents);
        if let Err(utf8_conversion_err) = output {
            println!("Error converting blob to utf8: {}", utf8_conversion_err);
            return Err(fmt::Error)
        } else {
            write!(f, "{}", output.unwrap())
        }
    }
}
