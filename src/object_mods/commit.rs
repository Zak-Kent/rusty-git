use nom::{
    bytes::complete::{is_not, take, take_till1, take_while1},
    character::{
        complete::{space0, space1},
        is_newline,
    },
    multi::many1,
    IResult,
};
use std::collections::HashMap;
use std::fmt;
use std::str::from_utf8;

use super::AsBytes;
use crate::error as err;

fn parse_kv_pair(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
    let (input, key) = is_not(" \t\r\n")(input)?;
    let (input, _) = space1(input)?;
    let (input, val) = take_till1(|c| c == b'\n')(input)?;
    let (input, _) = take(1usize)(input)?;
    return Ok((input, (key, val)));
}

fn parse_kv_pairs(input: &[u8]) -> IResult<&[u8], (Vec<Vec<u8>>, HashMap<Vec<u8>, Vec<u8>>)> {
    let (input, pairs) = many1(parse_kv_pair)(input)?;
    let mut kvs = HashMap::new();
    let mut insert_order = Vec::new();
    for (k, v) in pairs {
        insert_order.push(k.to_vec());
        kvs.insert(k.to_vec(), v.to_vec());
    }
    return Ok((input, (insert_order.to_owned(), kvs)));
}

fn parse_seperator_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = space0(input)?;
    let (input, nl) = take_while1(is_newline)(input)?;
    return Ok((input, nl));
}

// list of key value pairs with msg
// this format is used for commits and tags
#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub kvs: HashMap<Vec<u8>, Vec<u8>>,
    pub kvs_order: Vec<Vec<u8>>,
    pub msg: Vec<u8>,
    pub sha: String,
}

fn commit_body_as_bytes(commit: &Commit) -> Vec<u8> {
    let mut commit_bytes: Vec<u8> = Vec::new();
    for elm in commit.kvs_order.clone() {
        if let Some((k, v)) = commit.kvs.get_key_value(&elm) {
            let mut line: Vec<u8> = [k.clone(), [b' '].to_vec(), v.clone(), [b'\n'].to_vec()]
                .into_iter()
                .flatten()
                .collect();
            commit_bytes.append(&mut line);
        }
    }
    commit_bytes.push(b'\n');
    commit_bytes.append(&mut commit.msg.clone());
    return commit_bytes;
}

impl AsBytes for Commit {
    fn as_bytes(&self) -> Vec<u8> {
        let mut commit_body = commit_body_as_bytes(self);

        let mut output_bytes: Vec<u8> = [
            b"commit".to_vec(),
            [b' '].to_vec(),
            commit_body.len().to_string().as_bytes().to_vec(),
            [b'\x00'].to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        output_bytes.append(&mut commit_body);
        return output_bytes;
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let commit_bytes = commit_body_as_bytes(self);
        let output = from_utf8(&commit_bytes);
        if let Err(utf8_conversion_err) = output {
            println!("Error converting commit to utf8: {}", utf8_conversion_err);
            return Err(fmt::Error);
        } else {
            write!(f, "{}", output.unwrap())
        }
    }
}

pub fn parse_kv_list_msg(input: &[u8], sha: &str) -> Result<Commit, err::Error> {
    let (input, (kvs_order, kvs)) = parse_kv_pairs(input)?;
    let (input, _) = parse_seperator_line(input)?;
    return Ok(Commit {
        kvs,
        kvs_order,
        msg: input.to_vec(),
        sha: sha.to_owned(),
    });
}

#[cfg(test)]
mod commit_tests {
    use super::*;

    #[test]
    fn can_parse_kv_pair() {
        let kv_pair = "tree foobar\n".as_bytes();
        let (input, (k, v)) = parse_kv_pair(&kv_pair).unwrap();
        assert_eq!(k, "tree".as_bytes());
        assert_eq!(v, "foobar".as_bytes());
        assert!(input.is_empty());
    }

    #[test]
    fn can_parse_kv_pairs() {
        let kv_pairs = [
            "tree tree-val\n",
            "parent parent-val\n",
            "author author val\n",
        ]
        .map(|s| s.as_bytes())
        .concat();

        let (input, (pair_order, pairs)) = parse_kv_pairs(&kv_pairs).unwrap();
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
        assert_eq!(
            Vec::from(["tree".as_bytes(), "parent".as_bytes(), "author".as_bytes()]),
            pair_order
        );
    }

    #[test]
    fn can_parse_commit_msg() {
        let commit_msg = [
            "tree tree-val\n",
            "parent parent-val\n",
            "\n",
            "this is a test commit\n",
            "message",
        ]
        .map(|s| s.as_bytes())
        .concat();

        let fake_sha = "foobar";

        let Commit {
            kvs,
            kvs_order,
            msg,
            sha,
        } = parse_kv_list_msg(&commit_msg, &fake_sha).unwrap();

        assert_eq!(&"tree-val".as_bytes(), kvs.get("tree".as_bytes()).unwrap());
        assert_eq!(
            &"parent-val".as_bytes(),
            kvs.get("parent".as_bytes()).unwrap()
        );
        assert_eq!(2, kvs.keys().count());
        assert_eq!("this is a test commit\nmessage".as_bytes(), msg);
        assert_eq!("tree".as_bytes(), kvs_order[0]);
        assert_eq!("foobar", sha);
    }
}
