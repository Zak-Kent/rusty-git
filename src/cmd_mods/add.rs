use chrono::{TimeZone, Utc};
use std::fs::{metadata, File};
use std::io::Write;
use std::os::unix::prelude::MetadataExt;

use crate::error as err;
use crate::index as idx;
use crate::objects::{self as obj, blob, AsBytes};
use crate::utils;

pub fn file_to_index_entry(
    file_name: &str,
    repo: &obj::Repo,
) -> Result<idx::IndexEntry, err::Error> {
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

    let blob = blob::blob_from_path(file)?;
    let sha = obj::write_object(blob, None)?;

    Ok(idx::IndexEntry {
        c_time: c_time_dt,
        m_time: m_time_dt,
        dev: md.dev() as u32,
        inode: md.ino() as u32,
        mode: md.mode(),
        uid: md.uid(),
        gid: md.gid(),
        size: md.size() as u32,
        sha: sha.bytes().to_vec(),
        name: file_name.to_owned(),
    })
}

pub fn add_entry_to_index(repo: &obj::Repo, file_name: &str) -> Result<idx::Index, err::Error> {
    let index_contents = utils::git_read_index(repo)?;
    let mut index = idx::parse_git_index(&index_contents)?;

    let entry = file_to_index_entry(file_name, repo)?;
    match index.entries.binary_search(&entry) {
        // already exists, remove existing, replace with new
        Ok(pos) => {
            index.entries.remove(pos);
            index.entries.insert(pos, entry);
        }
        // doesn't exist, add at pos where entry should be
        Err(pos) => index.entries.insert(pos, entry),
    };
    Ok(index.to_owned())
}

pub fn write_index(index: idx::Index, repo: &obj::Repo) -> Result<(), err::Error> {
    // the File::create call will truncate the index
    let mut index_file = File::create(repo.gitdir.join("index"))?;
    index_file.write(&index.as_bytes())?;
    Ok(())
}

pub fn update_index(repo: &obj::Repo, file_name: &str) -> Result<(), err::Error> {
    let updated_index = add_entry_to_index(repo, file_name)?;
    write_index(updated_index, repo)?;
    Ok(())
}
