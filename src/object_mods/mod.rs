use inflate::inflate_bytes_zlib;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till1},
    character::{complete::space1, is_newline},
    error::{Error, ErrorKind},
    Err, IResult,
};
use std::fs::read;
use std::str::from_utf8;

use crate::error as err;
use crate::objects as obj;
use crate::utils;

pub mod blob;
pub mod commit;
pub mod tree;

pub trait NameSha {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String);
}

pub trait AsBytes {
    fn as_bytes(&self) -> Vec<u8>;
}

fn generic_nom_failure(input: &[u8]) -> Err<Error<&[u8]>> {
    Err::Failure(Error {
        input,
        code: ErrorKind::Fail,
    })
}

pub fn parse_git_head(input: &[u8]) -> Result<String, err::Error> {
    let (input, _key) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (_, head_ref) = take_till1(is_newline)(input)?;
    return Ok(from_utf8(head_ref)?.to_owned());
}

fn parse_obj_type<'a>(input: &'a [u8]) -> IResult<&'a [u8], obj::GitObjTyp> {
    let (input, obj) = alt((tag("blob"), tag("commit"), tag("tree")))(input)?;
    return match obj {
        b"blob" => Ok((input, obj::GitObjTyp::Blob)),
        b"commit" => Ok((input, obj::GitObjTyp::Commit)),
        b"tree" => Ok((input, obj::GitObjTyp::Tree)),
        _ => Err(generic_nom_failure(input)),
    };
}

fn parse_obj_len(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, _) = space1(input)?;
    let (input, size) = take_till1(|c| c == b'\x00')(input)?;
    let (input, _) = tag(b"\x00")(input)?;

    let str_num = match from_utf8(size) {
        Ok(s) => s,
        _ => return Err(generic_nom_failure(input)),
    };

    let output = match str_num.parse::<usize>() {
        Ok(n) => n,
        _ => return Err(generic_nom_failure(input)),
    };
    return Ok((input, output));
}

#[derive(Debug, PartialEq)]
pub enum GitObj {
    Blob(blob::Blob),
    Tree(tree::Tree),
    Commit(commit::KvsMsg),
}

pub fn parse_git_obj<'a>(input: &'a [u8], sha: &'a str) -> Result<GitObj, err::Error> {
    let (input, obj) = parse_obj_type(input)?;
    let (contents, len) = parse_obj_len(input)?;
    if len != contents.len() {
        return Err(err::Error::GitMalformedObject);
    }
    match obj {
        obj::GitObjTyp::Blob => Ok(GitObj::Blob(blob::Blob::new(contents))),
        obj::GitObjTyp::Tree => Ok(GitObj::Tree(tree::parse_git_tree(contents)?)),
        obj::GitObjTyp::Commit => Ok(GitObj::Commit(commit::parse_kv_list_msg(contents, sha)?)),
    }
}

pub fn read_object(sha: &str, repo: &obj::Repo) -> Result<GitObj, err::Error> {
    let obj_path = utils::git_obj_path_from_sha(sha, &repo)?;
    let contents = read(&obj_path)?;
    let decoded = match inflate_bytes_zlib(&contents) {
        Ok(res) => res,
        Err(e) => return Err(err::Error::InflatingGitObj(e)),
    };
    return Ok(parse_git_obj(&decoded, &sha)?);
}

pub fn read_object_as_string(sha: &str, repo: &obj::Repo) -> Result<String, err::Error> {
    let gitobject = read_object(sha, &repo)?;
    let obj_bytes = match gitobject {
        GitObj::Blob(blob) => blob.as_bytes(),
        GitObj::Tree(tree) => tree.as_bytes(),
        GitObj::Commit(commit) => commit.as_bytes(),
    };
    return Ok(from_utf8(&obj_bytes)?.to_owned());
}

#[cfg(test)]
mod object_mod_tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn can_parse_git_head() {
        let head_file = "ref: refs/heads/main".as_bytes();
        assert_eq!("refs/heads/main", parse_git_head(head_file).unwrap());
    }

    #[test]
    fn can_parse_git_blob() {
        let test_inflated_git_obj = ["blob 17", "\x00", "git file contents"]
            .map(|s| s.as_bytes())
            .concat();
        let sha = "abc123";
        if let GitObj::Blob(blob) = parse_git_obj(&test_inflated_git_obj, &sha).unwrap() {
            assert_eq!("git file contents", from_utf8(&blob.contents).unwrap());
            assert_eq!(17, blob.len);
        } else {
            panic!("should be a Blob object")
        }
    }

    #[test]
    fn can_round_trip_commit() {
        let commit_bytes = test_utils::fake_commit();
        let sha = "8f30e364422bba93030062297731f00a1510984b";
        if let GitObj::Commit(parsed_commit) = parse_git_obj(&commit_bytes, sha).unwrap() {
            let round_trip_commit = parsed_commit.as_bytes();
            assert_eq!(commit_bytes, round_trip_commit);
        } else {
            panic!("should be a Commit object")
        }
    }
}
