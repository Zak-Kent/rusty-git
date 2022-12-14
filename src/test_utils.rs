use std::fs::{read, File};
use std::io::{Error, Write};
use std::path::Path;
use tempfile::{tempdir, TempDir};

use crate::error as err;
use crate::object_parsers as objp;
use crate::objects as obj;
use crate::utils;

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
    utils::create_git_repo(dir.path())?;
    return Ok(dir);
}

#[allow(dead_code)]
pub fn test_gitdir_with_index() -> Result<TempDir, err::Error> {
    let dir = test_gitdir()?;
    let mut index = File::create(dir.path().join(".git/index"))?;
    index.write(&fake_index_bytes())?;
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
    let head_ref = objp::parse_git_head(&head)?;
    let mut ref_file = File::create(repo.gitdir.join(head_ref))?;
    writeln!(ref_file, "{}", sha)?;
    Ok(())
}

#[allow(dead_code)]
pub fn fake_index_bytes() -> Vec<u8> {
    [
        68, 73, 82, 67, 0, 0, 0, 2, 0, 0, 0, 8, 99, 134, 133, 151, 26, 198, 1, 77, 99, 134, 133,
        151, 26, 198, 1, 77, 1, 0, 0, 4, 0, 94, 162, 84, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20,
        0, 0, 1, 179, 119, 254, 94, 4, 37, 226, 247, 186, 101, 44, 84, 22, 59, 242, 131, 50, 148,
        86, 222, 57, 0, 10, 67, 97, 114, 103, 111, 46, 116, 111, 109, 108, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 129, 164, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 55, 228, 89, 60, 218, 13, 59, 253, 59, 115, 25, 117,
        147, 194, 253, 192, 76, 197, 30, 0, 10, 115, 114, 99, 47, 99, 108, 105, 46, 114, 115, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 248, 47, 249, 197, 177, 18, 227, 7,
        129, 105, 212, 73, 244, 161, 101, 162, 57, 109, 211, 250, 0, 15, 115, 114, 99, 47, 99, 111,
        109, 109, 97, 110, 100, 115, 46, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16,
        128, 130, 85, 252, 207, 120, 222, 224, 14, 24, 245, 101, 253, 250, 193, 213, 243, 105, 43,
        0, 12, 115, 114, 99, 47, 101, 114, 114, 111, 114, 46, 114, 115, 0, 0, 0, 0, 0, 0, 99, 135,
        164, 129, 40, 162, 231, 102, 99, 135, 164, 129, 40, 162, 231, 102, 1, 0, 0, 4, 0, 95, 21,
        64, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 1, 242, 35, 27, 120, 172, 253, 99, 23,
        237, 3, 87, 13, 1, 115, 221, 32, 73, 54, 108, 72, 108, 0, 11, 115, 114, 99, 47, 109, 97,
        105, 110, 46, 114, 115, 0, 0, 0, 0, 0, 0, 0, 99, 135, 164, 129, 40, 172, 131, 113, 99, 135,
        164, 129, 40, 172, 131, 113, 1, 0, 0, 4, 0, 95, 21, 65, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0,
        0, 20, 0, 0, 56, 238, 144, 94, 234, 103, 52, 105, 103, 149, 85, 165, 88, 40, 124, 88, 147,
        188, 98, 39, 214, 61, 0, 21, 115, 114, 99, 47, 111, 98, 106, 101, 99, 116, 95, 112, 97,
        114, 115, 101, 114, 115, 46, 114, 115, 0, 0, 0, 0, 0, 99, 118, 129, 163, 44, 243, 27, 162,
        99, 118, 129, 163, 44, 243, 27, 162, 1, 0, 0, 4, 0, 90, 191, 65, 0, 0, 129, 164, 0, 0, 1,
        245, 0, 0, 0, 20, 0, 0, 27, 81, 242, 62, 90, 226, 216, 80, 134, 183, 122, 28, 135, 5, 147,
        88, 112, 113, 51, 144, 147, 41, 0, 14, 115, 114, 99, 47, 111, 98, 106, 101, 99, 116, 115,
        46, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 94, 245, 149, 200, 225, 77, 104,
        173, 220, 22, 241, 0, 166, 218, 97, 147, 50, 113, 102, 91, 0, 12, 115, 114, 99, 47, 117,
        116, 105, 108, 115, 46, 114, 115, 0, 0, 0, 0, 0, 0, 84, 82, 69, 69, 0, 0, 0, 53, 0, 56, 32,
        49, 10, 94, 80, 75, 81, 83, 56, 251, 27, 118, 251, 44, 61, 74, 48, 123, 44, 209, 219, 24,
        88, 115, 114, 99, 0, 55, 32, 48, 10, 247, 119, 37, 9, 236, 45, 66, 113, 190, 230, 234, 87,
        91, 155, 125, 203, 198, 212, 185, 70, 164, 227, 215, 175, 119, 29, 118, 67, 66, 89, 140,
        127, 94, 30, 181, 10, 76, 188, 194, 142,
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
