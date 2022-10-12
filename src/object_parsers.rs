use nom::branch::alt;
use nom::{
    bytes::complete::{tag, take_till1},
    character::complete::space1,
    error::{Error, ErrorKind},
    Err, IResult,
};

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
}
