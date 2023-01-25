pub mod tree;

pub trait NameSha {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String);
}

pub trait AsBytes {
    fn as_bytes(&self) -> Vec<u8>;
}
