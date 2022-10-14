use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1, take_while, take_while1},
    character::{
        complete::{space0, space1},
        is_newline,
    },
    error::{Error, ErrorKind},
    multi::many1,
    Err, IResult,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::from_utf8;

use crate::error as err;
use crate::objects as obj;

// TODO: figure out a way to make nom errors more specific
fn generic_nom_err(input: &[u8]) -> Err<Error<&[u8]>> {
    Err::Failure(Error {
        input,
        code: ErrorKind::Fail,
    })
}

fn parse_obj_type<'a>(input: &'a [u8], path: &'a PathBuf) -> IResult<&'a [u8], obj::GitObject> {
    let (input, obj) = alt((tag("blob"), tag("commit"), tag("tree")))(input)?;
    return match obj {
        b"blob" => Ok((input, obj::GitObject::Blob(path.to_path_buf()))),
        b"commit" => Ok((input, obj::GitObject::Commit)),
        b"tree" => Ok((input, obj::GitObject::Tree)),
        _ => Err(generic_nom_err(input)),
    };
}

fn parse_obj_len(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, _) = space1(input)?;
    let (input, size) = take_till1(|c| c == b'\x00')(input)?;
    let (input, _) = tag(b"\x00")(input)?;

    let str_num = match from_utf8(size) {
        Ok(s) => s,
        _ => return Err(generic_nom_err(input)),
    };

    let output = match str_num.parse::<usize>() {
        Ok(n) => n,
        _ => return Err(generic_nom_err(input)),
    };
    return Ok((input, output));
}

pub fn parse_git_obj<'a>(
    input: &'a [u8],
    path: &'a PathBuf,
) -> Result<obj::GitObjInfo<'a>, err::Error> {
    let (input, obj) = parse_obj_type(input, path)?;
    let (input, len) = parse_obj_len(input)?;
    return Ok(obj::GitObjInfo {
        obj,
        len,
        contents: input,
    });
}

fn parse_kv_pair(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
    let (input, key) = take_till1(|c| c == b' ')(input)?;
    let (input, _) = space1(input)?;
    let (input, val) = take_till1(|c| c == b'\n')(input)?;
    let (input, _) = take_while(is_newline)(input)?;
    return Ok((input, (key, val)));
}

fn parse_kv_pairs(input: &[u8]) -> IResult<&[u8], HashMap<&[u8], &[u8]>> {
    let (input, pairs) = many1(parse_kv_pair)(input)?;
    let mut kvs = HashMap::new();
    for (k, v) in pairs {
        kvs.insert(k, v);
    }
    return Ok((input, kvs));
}

fn parse_seperator_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = space0(input)?;
    let (input, nl) = take_while1(is_newline)(input)?;
    return Ok((input, nl));
}

// list of key value pairs with msg
// this format is used for commits and tags
struct KvsMsg<'a> {
    kvs: HashMap<&'a [u8], &'a [u8]>,
    msg: &'a [u8],
}

fn parse_kv_list_msg(input: &[u8]) -> Result<KvsMsg, err::Error> {
    let (input, kvs) = parse_kv_pairs(input)?;
    let (input, _) = parse_seperator_line(input)?;
    return Ok(KvsMsg { kvs, msg: input });
}

#[cfg(test)]
mod object_parsing_tests {
    use super::*;

    #[test]
    fn can_parse_git_object() {
        let test_inflated_git_objt = [
            "blob".as_bytes(),
            " ".as_bytes(),
            "12".as_bytes(),
            "\x00".as_bytes(),
            "git file contents".as_bytes(),
        ]
        .concat();
        let path = PathBuf::from("foo/path");

        let gitobjinfo = parse_git_obj(&test_inflated_git_objt, &path).unwrap();

        assert_eq!("git file contents", from_utf8(gitobjinfo.contents).unwrap());
        assert_eq!(12, gitobjinfo.len);
        assert_eq!(
            obj::GitObject::Blob(PathBuf::from("foo/path")),
            gitobjinfo.obj
        );
    }

    #[test]
    fn can_parse_kv_pair() {
        let kv_pair = [
            "tree".as_bytes(),
            " ".as_bytes(),
            "foobar".as_bytes(),
            "\n".as_bytes(),
        ]
        .concat();
        let (input, (k, v)) = parse_kv_pair(&kv_pair).unwrap();
        assert_eq!(k, "tree".as_bytes());
        assert_eq!(v, "foobar".as_bytes());
        assert!(input.is_empty());
    }

    #[test]
    fn can_parse_kv_pairs() {
        let kv_pairs = [
            "tree".as_bytes(),
            " ".as_bytes(),
            "tree-val".as_bytes(),
            "\n".as_bytes(),
            "parent".as_bytes(),
            " ".as_bytes(),
            "parent-val".as_bytes(),
            "\n".as_bytes(),
            "author".as_bytes(),
            " ".as_bytes(),
            "author val".as_bytes(),
            "\n".as_bytes(),
        ]
        .concat();

        let (input, pairs) = parse_kv_pairs(&kv_pairs).unwrap();
        assert_eq!(
            &"tree-val".as_bytes(),
            pairs.get("tree".as_bytes()).unwrap()
        );
        assert_eq!(
            &"parent-val".as_bytes(),
            pairs.get("parent".as_bytes()).unwrap()
        );
        assert_eq!(
            &"author val".as_bytes(),
            pairs.get("author".as_bytes()).unwrap()
        );
        assert!(input.is_empty());
    }

    #[test]
    fn can_parse_commit_msg() {
        let commit_msg = [
            "tree".as_bytes(),
            " ".as_bytes(),
            "tree-val".as_bytes(),
            "\n".as_bytes(),
            "parent".as_bytes(),
            " ".as_bytes(),
            "parent-val".as_bytes(),
            "\n".as_bytes(),
            "             \n".as_bytes(),
            "this is a test commit\n".as_bytes(),
            "message".as_bytes(),
        ]
        .concat();

        let KvsMsg { kvs, msg } = parse_kv_list_msg(&commit_msg).unwrap();
        assert_eq!(&"tree-val".as_bytes(), kvs.get("tree".as_bytes()).unwrap());
        assert_eq!(
            &"parent-val".as_bytes(),
            kvs.get("parent".as_bytes()).unwrap()
        );
        assert_eq!(2, kvs.keys().count());
        assert_eq!("this is a test commit\nmessage".as_bytes(), msg);
    }
}
