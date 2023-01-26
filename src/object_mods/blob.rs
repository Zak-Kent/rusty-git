use super::AsBytes;

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
        return self.contents.clone();
    }
}
