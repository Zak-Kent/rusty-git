use chrono::{TimeZone, Utc};
use std::fs::{metadata, read, read_dir, read_to_string, File};
use std::io::Write;
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};
use std::str::from_utf8;

use crate::error as err;
use crate::object_parsers::{self as objp, ToBinary};
use crate::objects as obj;


// ----------- git utils ---------------
pub fn is_git_repo(path: &Path) -> bool {
    let gitdir = path.join(".git");
    let conf = path.join(".git/config");
    gitdir.exists() && conf.exists()
}

pub fn git_repo_or_err(path: &Path) -> Result<PathBuf, err::Error> {
    let gitrepo = is_git_repo(path);
    if gitrepo {
        return Ok(path.to_owned());
    } else {
        Err(err::Error::GitNotARepo)
    }
}

pub fn git_obj_path_from_sha(sha: &str, repo: &obj::Repo) -> Result<PathBuf, err::Error> {
    let obj_path = repo
        .gitdir
        .join(format!("objects/{}/{}", &sha[..2], &sha[2..]));

    if obj_path.exists() {
        return Ok(obj_path.to_path_buf());
    } else {
        return Err(err::Error::GitObjPathDoesntExist(
            obj_path.display().to_string(),
        ));
    }
}

pub fn git_sha_from_head(repo: &obj::Repo) -> Result<String, err::Error> {
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = objp::parse_git_head(&head)?;
    let sha_path = repo.gitdir.join(head_ref);

    if sha_path.exists() {
        let sha = read_to_string(&sha_path)?.trim().to_owned();
        return Ok(sha);
    } else {
        return Err(err::Error::GitNoCommitsExistYet);
    };
}

pub fn git_get_tree_from_commit(
    sha: &str,
    contents: &[u8],
    repo: &obj::Repo,
) -> Result<objp::Tree, err::Error> {
    let objp::KvsMsg { kvs, .. } = objp::parse_kv_list_msg(contents, sha)?;

    let tree_sha = match kvs.get("tree".as_bytes()) {
        Some(s) => from_utf8(s)?,
        None => return Err(err::Error::GitNoTreeKeyInCommit),
    };

    let obj::GitObject { contents, .. } = obj::read_object(tree_sha, repo)?;
    let tree = objp::parse_git_tree(&contents)?;

    return Ok(tree);
}

pub fn git_read_index(repo: &obj::Repo) -> Result<Vec<u8>, err::Error> {
    let index_path = repo.gitdir.join("index");
    return Ok(read(index_path)?);
}

pub fn git_index_exists(repo: &obj::Repo) -> bool {
    return repo.gitdir.clone().join("index").exists();
}

pub fn git_check_for_rusty_git_allowed(repo: &obj::Repo) -> Result<bool, err::Error> {
    let work_path = repo.worktree.clone();
    let worktree_dir = read_dir(work_path)?;
    let mut rusty_git_allowed = false;

    for node in worktree_dir {
        let node_val = node?;
        let node_name = node_val.file_name();
        if node_name == ".rusty-git-allowed" {
            rusty_git_allowed = true;
        }
    }

    if rusty_git_allowed {
        return Ok(rusty_git_allowed);
    } else {
        return Err(err::Error::RustyGitAllowedFileMissing);
    }
}

pub fn git_file_to_index_entry(
    file_name: &str,
    repo: &obj::Repo,
) -> Result<objp::IndexEntry, err::Error> {
    let file = repo.worktree.join(file_name);
    let md = metadata(&file)?;

    let c_time_dt;
    if let Some(ct) = Utc
        .timestamp_opt(md.ctime().into(), md.ctime_nsec() as u32)
        .single()
    {
        c_time_dt = ct;
    } else {
        return Err(err::Error::TimestampConversionError);
    };

    let m_time_dt;
    if let Some(mt) = Utc
        .timestamp_opt(md.ctime().into(), md.ctime_nsec() as u32)
        .single()
    {
        m_time_dt = mt;
    } else {
        return Err(err::Error::TimestampConversionError);
    };

    let hash = obj::write_object(
        obj::SourceFile {
            typ: obj::GitObjTyp::Blob,
            source: file,
        },
        None,
    )?;

    return Ok(objp::IndexEntry {
        c_time: c_time_dt,
        m_time: m_time_dt,
        dev: md.dev() as u32,
        inode: md.ino() as u32,
        mode: md.mode(),
        uid: md.uid(),
        gid: md.gid(),
        size: md.size() as u32,
        sha: hash.bytes().to_vec(),
        name: file_name.to_owned(),
    });
}

pub fn git_add_entry_to_index(
    repo: &obj::Repo,
    file_name: &str,
) -> Result<objp::Index, err::Error> {
    let index_contents = git_read_index(repo)?;
    let mut index = objp::parse_git_index(&index_contents)?;

    let entry = git_file_to_index_entry(file_name, repo)?;
    match index.entries.binary_search(&entry) {
        // already exists, remove existing, replace with new
        Ok(pos) => {
            index.entries.remove(pos);
            index.entries.insert(pos, entry);
        }
        // doesn't exist, add at pos where entry should be
        Err(pos) => index.entries.insert(pos, entry),
    };
    return Ok(index.to_owned());
}

pub fn git_write_index(index: objp::Index, repo: &obj::Repo) -> Result<(), err::Error> {
    // the File::create call will truncate the index
    let mut index_file = File::create(repo.gitdir.join("index"))?;
    index_file.write(&index.to_binary())?;
    return Ok(());
}

pub fn git_update_index(repo: &obj::Repo, file_name: &str) -> Result<(), err::Error> {
    let updated_index = git_add_entry_to_index(repo, file_name)?;
    git_write_index(updated_index, repo)?;
    return Ok(());
}

// ----------- fs utils ---------------
pub fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

pub fn content_length(path: &Path) -> Result<u64, err::Error> {
    Ok(metadata(path)?.len())
}

#[cfg(test)]
mod utils_tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn return_true_when_git_repo() {
        // need the two var decs below to get around the borrow checker
        // not seeing that the ref .path() creates should be bound
        // through the unwrap when written: Result<TempDir>.unwrap().path()
        let gitdir = test_utils::test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path))
    }

    #[test]
    fn return_false_when_not_git_repo() {
        let tempdir = test_utils::test_tempdir().unwrap();
        let tempdir_path = tempdir.path();
        assert!(!is_git_repo(tempdir_path))
    }

    #[test]
    fn dir_is_empty_works_as_expected() {
        let tempdir = test_utils::test_tempdir().unwrap();
        let gitdir = test_utils::test_gitdir().unwrap();
        assert_eq!(Ok(true), test_utils::dir_is_empty(tempdir.path()));
        assert_eq!(Ok(false), test_utils::dir_is_empty(gitdir.path()));
    }
}
