use std::fs::{metadata, read, File};
use std::io::{Error, Write};
use std::path::Path;
use tempfile::{tempdir, TempDir};

use crate::cmd_mods::init;
use crate::error as err;
use crate::objects as obj;

#[allow(dead_code)]
pub fn dir_is_empty(path: &Path) -> Result<bool, err::Error> {
    return Ok(path.try_exists()? && path.read_dir()?.next().is_none());
}

#[allow(dead_code)]
pub fn test_tempdir() -> Result<TempDir, Error> {
    let tmp_dir = tempdir()?;
    return Ok(tmp_dir);
}

#[allow(dead_code)]
pub fn test_gitdir() -> Result<TempDir, err::Error> {
    let dir = test_tempdir()?;
    init::create_git_repo(dir.path())?;
    return Ok(dir);
}

#[allow(dead_code)]
pub fn test_gitdir_with_index() -> Result<TempDir, err::Error> {
    let dir = test_gitdir()?;
    let mut index = File::create(dir.path().join(".git/index"))?;
    index.write(&fake_index_without_extension_info())?;
    return Ok(dir);
}

#[allow(dead_code)]
pub fn test_add_dummy_commit_and_update_ref_heads(
    sha: &str,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    //TODO: expand this to add an actual commit in .git/objects later
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = obj::parse_git_head(&head)?;
    let mut ref_file = File::create(repo.gitdir.join(head_ref))?;
    writeln!(ref_file, "{}", sha)?;
    Ok(())
}

#[allow(dead_code)]
pub fn content_length(path: &Path) -> Result<u64, err::Error> {
    Ok(metadata(path)?.len())
}

#[allow(dead_code)]
pub fn fake_index_without_extension_info() -> Vec<u8> {
    [
        68, 73, 82, 67, 0, 0, 0, 2, 0, 0, 0, 4, 99, 210, 241, 47, 45, 150, 143, 255, 99, 210, 241,
        47, 45, 150, 143, 255, 1, 0, 0, 4, 0, 110, 210, 190, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0,
        20, 0, 0, 0, 6, 208, 125, 111, 249, 39, 127, 143, 3, 95, 190, 187, 59, 58, 64, 179, 27, 93,
        35, 36, 131, 0, 7, 98, 97, 114, 46, 116, 120, 116, 0, 0, 0, 99, 212, 40, 199, 8, 21, 171,
        27, 99, 212, 40, 199, 8, 21, 171, 27, 1, 0, 0, 4, 0, 111, 86, 248, 0, 0, 129, 164, 0, 0, 1,
        245, 0, 0, 0, 20, 0, 0, 0, 38, 227, 178, 167, 20, 132, 91, 12, 21, 55, 250, 126, 164, 143,
        102, 126, 124, 32, 149, 158, 111, 0, 8, 99, 101, 108, 116, 46, 116, 120, 116, 0, 0, 99,
        211, 249, 24, 21, 254, 29, 55, 99, 211, 249, 24, 21, 254, 29, 55, 1, 0, 0, 4, 0, 111, 86,
        228, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 0, 22, 135, 122, 109, 196, 91, 171,
        44, 25, 9, 4, 152, 23, 101, 162, 217, 53, 128, 25, 161, 46, 0, 8, 100, 101, 108, 116, 46,
        116, 120, 116, 0, 0, 99, 210, 242, 213, 32, 61, 191, 14, 99, 210, 242, 213, 32, 61, 191,
        14, 1, 0, 0, 4, 0, 110, 210, 120, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 0, 11,
        84, 190, 33, 13, 161, 48, 224, 226, 169, 159, 206, 54, 130, 128, 72, 129, 83, 231, 153, 53,
        0, 7, 102, 111, 111, 46, 116, 120, 116, 0, 0, 0, 112, 59, 135, 70, 216, 84, 84, 3, 171, 19,
        244, 118, 11, 60, 85, 251, 247, 43, 68, 127,
    ]
    .to_vec()
}

#[allow(dead_code)]
pub fn fake_index_entry() -> Vec<u8> {
    [
        99, 134, 102, 238, 3, 187, 189, 180, 99, 134, 102, 238, 3, 187, 189, 180, 1, 0, 0, 4, 0,
        94, 104, 237, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 1, 179, 119, 254, 94, 4, 37,
        226, 247, 186, 101, 44, 84, 22, 59, 242, 131, 50, 148, 86, 222, 57, 0, 10, 67, 97, 114,
        103, 111, 46, 116, 111, 109, 108, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
    .to_vec()
}

#[allow(dead_code)]
pub fn fake_index_no_entry() -> Vec<u8> {
    [
        68, 73, 82, 67, 0, 0, 0, 2, 0, 0, 0, 0, 57, 216, 144, 19, 158, 229, 53, 108, 126, 245, 114,
        33, 108, 235, 205, 39, 170, 65, 249, 223,
    ]
    .to_vec()
}

#[allow(dead_code)]
pub fn fake_commit() -> Vec<u8> {
    [
        99, 111, 109, 109, 105, 116, 32, 49, 54, 50, 0, 116, 114, 101, 101, 32, 48, 57, 97, 49, 51,
        98, 56, 57, 55, 100, 51, 100, 48, 102, 53, 50, 56, 100, 52, 56, 55, 99, 55, 48, 52, 100,
        97, 53, 52, 48, 99, 98, 57, 53, 50, 100, 55, 54, 48, 54, 10, 97, 117, 116, 104, 111, 114,
        32, 90, 97, 107, 45, 75, 101, 110, 116, 32, 60, 122, 97, 107, 46, 107, 101, 110, 116, 64,
        103, 109, 97, 105, 108, 46, 99, 111, 109, 62, 32, 49, 54, 55, 51, 52, 55, 48, 54, 50, 56,
        32, 45, 48, 55, 48, 48, 10, 99, 111, 109, 109, 105, 116, 116, 101, 114, 32, 90, 97, 107,
        45, 75, 101, 110, 116, 32, 60, 122, 97, 107, 46, 107, 101, 110, 116, 64, 103, 109, 97, 105,
        108, 46, 99, 111, 109, 62, 32, 49, 54, 55, 51, 52, 55, 48, 54, 50, 56, 32, 45, 48, 55, 48,
        48, 10, 10, 102, 111, 111, 10,
    ]
    .to_vec()
}
