use nom::{
    bits::complete::take as bit_take,
    branch::alt,
    bytes::complete::{is_not, tag, take, take_till1, take_while1},
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

fn parse_obj_type<'a>(input: &'a [u8], path: &'a PathBuf) -> IResult<&'a [u8], obj::GitObjTyp> {
    let (input, obj) = alt((tag("blob"), tag("commit"), tag("tree")))(input)?;
    return match obj {
        b"blob" => Ok((input, obj::GitObjTyp::Blob)),
        b"commit" => Ok((input, obj::GitObjTyp::Commit)),
        b"tree" => Ok((input, obj::GitObjTyp::Tree)),
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
    sha: &'a str,
) -> Result<obj::GitObject, err::Error> {
    let (input, obj) = parse_obj_type(input, path)?;
    let (contents, len) = parse_obj_len(input)?;
    return Ok(obj::GitObject {
        obj,
        len,
        contents: contents.to_vec(),
        source: path.to_path_buf(),
        sha: sha.to_owned(),
    });
}

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
#[derive(Debug, Clone)]
pub struct KvsMsg {
    pub kvs: HashMap<Vec<u8>, Vec<u8>>,
    pub kvs_order: Vec<Vec<u8>>,
    pub msg: Vec<u8>,
    pub sha: String,
}

pub fn parse_kv_list_msg(input: &[u8], sha: &str) -> Result<KvsMsg, err::Error> {
    let (input, (kvs_order, kvs)) = parse_kv_pairs(input)?;
    let (input, _) = parse_seperator_line(input)?;
    return Ok(KvsMsg {
        kvs,
        kvs_order,
        msg: input.to_vec(),
        sha: sha.to_owned(),
    });
}

pub fn parse_git_head(input: &[u8]) -> Result<String, err::Error> {
    let (input, _key) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (_, head_ref) = take_till1(is_newline)(input)?;
    return Ok(from_utf8(head_ref)?.to_owned());
}

fn get_sha_from_binary(input: &[u8]) -> String {
    let mut hexpairs = Vec::new();
    for n in input {
        hexpairs.push(format!("{:02x}", n))
    }
    return hexpairs.join("");
}

// a single entry in a GitObjType::Tree file
type ParsedLeaf<'a> = (&'a [u8], &'a [u8], String);

pub fn parse_git_tree_leaf(input: &[u8]) -> IResult<&[u8], ParsedLeaf> {
    let (input, mode) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (input, path) = take_till1(|c| c == b'\x00')(input)?;
    let (input, _) = tag(b"\x00")(input)?;
    let (input, bsha) = take(20usize)(input)?;
    let sha = get_sha_from_binary(bsha);
    return Ok((input, ParsedLeaf::from((mode, path, sha))));
}

#[derive(Debug, PartialEq)]
pub struct TreeLeaf {
    pub mode: String,
    pub path: String,
    pub sha: String,
}

#[derive(Debug, PartialEq)]
pub struct Tree {
    pub contents: Vec<TreeLeaf>,
}

pub fn parse_git_tree(input: &[u8]) -> Result<Tree, err::Error> {
    let (_, leaves) = many1(parse_git_tree_leaf)(input)?;
    let mut contents: Vec<TreeLeaf> = Vec::new();

    for (mode, path, sha) in leaves {
        contents.push(TreeLeaf {
            mode: from_utf8(mode)?.to_owned(),
            path: from_utf8(path)?.to_owned(),
            sha,
        })
    }

    return Ok(Tree { contents });
}

type BitInput<'a> = (&'a [u8], usize);

fn take_n_bits(input: BitInput, count: u8) -> IResult<BitInput, u8> {
    bit_take(count)(input)
}

fn parse_num_to_mode(input: u32) -> Result<String, err::Error> {
    let byte_input = (input as u16).to_be_bytes();
    // this tuple is how Nom bit parsers keep track of where they are in the bytes
    let bit_input: BitInput = (&byte_input, 0);
    let (bit_input, file_type) = take_n_bits(bit_input, 3)?;
    let (bit_input, _) = take_n_bits(bit_input, 4)?;
    let (bit_input, user) = take_n_bits(bit_input, 3)?;
    let (bit_input, group) = take_n_bits(bit_input, 3)?;
    let (_, other) = take_n_bits(bit_input, 3)?;
    return Ok(format!("{:03b}{}{}{}", file_type, user, group, other));
}

#[cfg(test)]
mod object_parsing_tests {
    use super::*;
    use hex;
    use sha1_smol as sha1;

    #[test]
    fn can_parse_git_object() {
        let test_inflated_git_obj = ["blob 12", "\x00", "git file contents"]
            .map(|s| s.as_bytes())
            .concat();
        let path = PathBuf::from("foo/path");
        let sha = "abc123";
        let gitobject = parse_git_obj(&test_inflated_git_obj, &path, &sha).unwrap();
        assert_eq!("git file contents", from_utf8(&gitobject.contents).unwrap());
        assert_eq!(12, gitobject.len);
        assert_eq!(obj::GitObjTyp::Blob, gitobject.obj);
        assert_eq!("abc123".to_owned(), gitobject.sha);
    }

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

        let KvsMsg {
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

    #[test]
    fn can_parse_git_head() {
        let head_file = "ref: refs/heads/main".as_bytes();
        assert_eq!("refs/heads/main", parse_git_head(head_file).unwrap());
    }

    fn make_git_tree_leaf(file_name: &str, perms: &str) -> Vec<u8> {
        let file_info = [perms, " ", file_name, "\x00"]
            .map(|s| s.as_bytes())
            .concat();

        let mut hasher = sha1::Sha1::new();
        hasher.update(file_name.as_bytes());
        let sha = hasher.digest().to_string();
        let bsha = hex::decode(sha).unwrap();

        let mut leaf: Vec<u8> = Vec::new();
        leaf.extend_from_slice(&file_info);
        leaf.extend_from_slice(&bsha);
        return leaf;
    }

    #[test]
    fn can_parse_git_tree_leaf() {
        let leaf = make_git_tree_leaf("src/foo.txt", "100644");
        let expected_val = ParsedLeaf::from((
            b"100644",
            b"src/foo.txt",
            "73f73b8475d38e918a51739bf0e90dfba405f8af".to_owned(),
        ));
        let (leftover, leafvals) = parse_git_tree_leaf(&leaf).unwrap();
        assert_eq!(expected_val, leafvals);
        assert_eq!(0, leftover.len());
    }

    #[test]
    fn can_parse_git_tree_file() {
        let tree_file = [
            ("src/foo.txt", "100644"),
            ("tests", "040000"),
            ("src/bar.txt", "100644"),
        ]
        .map(|(f, p)| make_git_tree_leaf(f, p))
        .concat();

        let expected_val = Tree {
            contents: Vec::from([
                TreeLeaf {
                    mode: "100644".to_owned(),
                    path: "src/foo.txt".to_owned(),
                    sha: "73f73b8475d38e918a51739bf0e90dfba405f8af".to_owned(),
                },
                TreeLeaf {
                    mode: "040000".to_owned(),
                    path: "tests".to_owned(),
                    sha: "04d13fd0aa6f0197cf2c999019a607c36c81eb9f".to_owned(),
                },
                TreeLeaf {
                    mode: "100644".to_owned(),
                    path: "src/bar.txt".to_owned(),
                    sha: "df6a2dfaf9a69ddfc7d325031206f0d1895e1806".to_owned(),
                },
            ]),
        };
        let tree = parse_git_tree(&tree_file).unwrap();
        assert_eq!(expected_val, tree);
    }

    #[test]
    fn can_parse_mode_num() {
        let mode_num = 33188 as u32;
        let expected = "100644";
        let result = parse_num_to_mode(mode_num).unwrap();
        assert_eq!(expected, result);
    }
}
