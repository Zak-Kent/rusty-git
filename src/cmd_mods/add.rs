use chrono::{TimeZone, Utc};
use std::fs::{metadata, File};
use std::io::Write;
use std::os::unix::prelude::MetadataExt;

use crate::error as err;
use crate::object_parsers::{self as objp, ToBinary};
use crate::objects as obj;
use crate::utils;

pub fn file_to_index_entry(
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

pub fn add_entry_to_index(
    repo: &obj::Repo,
    file_name: &str,
) -> Result<objp::Index, err::Error> {
    let index_contents = utils::git_read_index(repo)?;
    let mut index = objp::parse_git_index(&index_contents)?;

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
    return Ok(index.to_owned());
}

pub fn write_index(index: objp::Index, repo: &obj::Repo) -> Result<(), err::Error> {
    // the File::create call will truncate the index
    let mut index_file = File::create(repo.gitdir.join("index"))?;
    index_file.write(&index.to_binary())?;
    return Ok(());
}

pub fn update_index(repo: &obj::Repo, file_name: &str) -> Result<(), err::Error> {
    let updated_index = add_entry_to_index(repo, file_name)?;
    write_index(updated_index, repo)?;
    return Ok(());
}
