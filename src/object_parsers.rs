use chrono::{DateTime, TimeZone, Utc};
use nom::{
    bits::complete::take as bit_take,
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take, take_till1, take_while1},
    character::{
        complete::{space0, space1},
        is_newline,
    },
    error::{Error, ErrorKind},
    multi::many1,
    number::{
        complete::{u16, u32},
        Endianness::Big,
    },
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

pub trait NameSha {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String);
}

#[derive(Debug, PartialEq)]
pub struct TreeLeaf {
    pub mode: String,
    pub path: String,
    pub sha: String,
}

impl NameSha for TreeLeaf {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String) {
        if let Some(prefix) = name_prefix {
            return (format!("{prefix}/{}", self.path), self.sha.clone())
        } else  {
            return (self.path.clone(), self.sha.clone());
        }
    }
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

// ------------- git index file parsers -----------------

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

#[derive(Debug, PartialEq)]
pub struct IndexEntry {
    pub c_time: DateTime<Utc>,
    pub m_time: DateTime<Utc>,
    pub dev: u32,
    pub inode: u32,
    pub mode: String,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub sha: String,
    pub name: String,
}

impl NameSha for IndexEntry {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String) {
        if let Some(prefix) = name_prefix {
            return (format!("{prefix}/{}", self.name), self.sha.clone())
        } else  {
            return (self.name.clone(), self.sha.clone());
        }
    }
}
pub fn parse_git_index_entry(input: &[u8]) -> IResult<&[u8], IndexEntry> {
    let (input, c_time) = u32(Big)(input)?;
    let (input, c_time_nano) = u32(Big)(input)?;
    let c_time_dt;
    if let Some(ct) = Utc.timestamp_opt(c_time.into(), c_time_nano).single() {
        c_time_dt = ct;
    } else {
        return Err(generic_nom_err(input));
    };

    let (input, m_time) = u32(Big)(input)?;
    let (input, m_time_nano) = u32(Big)(input)?;
    let m_time_dt;
    if let Some(mt) = Utc.timestamp_opt(m_time.into(), m_time_nano).single() {
        m_time_dt = mt;
    } else {
        return Err(generic_nom_err(input));
    };

    let (input, dev) = u32(Big)(input)?;
    let (input, inode) = u32(Big)(input)?;

    let (input, mode) = u32(Big)(input)?;
    let parsed_mode;
    if let Ok(pm) = parse_num_to_mode(mode) {
        parsed_mode = pm;
    } else {
        return Err(generic_nom_err(input));
    }

    let (input, uid) = u32(Big)(input)?;
    let (input, gid) = u32(Big)(input)?;
    let (input, size) = u32(Big)(input)?;
    let (input, bsha) = take(20usize)(input)?;
    let (input, name_size) = u16(Big)(input)?;

    let (input, name) = take(name_size)(input)?;
    let parsed_name;
    if let Ok(pn) = from_utf8(name) {
        parsed_name = pn;
    } else {
        return Err(generic_nom_err(input));
    }

    // 62 bytes per entry not counting length of name
    let entry_length = 62 + name_size;
    let padding_bytes = 8 - entry_length % 8;
    // the parser need to eat the padding bytes after each entry
    let (input, _null_bytes) = take(padding_bytes)(input)?;

    return Ok((
        input,
        IndexEntry {
            c_time: c_time_dt,
            m_time: m_time_dt,
            dev,
            inode,
            mode: parsed_mode,
            uid,
            gid,
            size,
            sha: get_sha_from_binary(bsha),
            name: parsed_name.to_owned(),
        },
    ));
}

#[derive(Debug)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

pub fn parse_git_index(input: &[u8]) -> Result<Index, err::Error> {
    let (input, _dirc) = is_a("DIRC")(input)?;
    let (input, version) = u32(Big)(input)?;
    if version != 2 {
        return Err(err::Error::GitUnrecognizedIndexVersion(version));
    }
    let (input, _num_entries) = u32(Big)(input)?;
    // expects at least 1 file in the index
    let (_, entries) = many1(parse_git_index_entry)(input)?;
    return Ok(Index { entries });
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

    #[test]
    fn can_parse_index_entry() {
        let entry = [
            99, 134, 102, 238, 3, 187, 189, 180, 99, 134, 102, 238, 3, 187, 189, 180, 1, 0, 0, 4,
            0, 94, 104, 237, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 1, 179, 119, 254, 94,
            4, 37, 226, 247, 186, 101, 44, 84, 22, 59, 242, 131, 50, 148, 86, 222, 57, 0, 10, 67,
            97, 114, 103, 111, 46, 116, 111, 109, 108, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let c_time = 1669752558;
        let c_time_nano = 62635444;
        let m_time = 1669752558;
        let m_time_nano = 62635444;

        let expected = IndexEntry {
            c_time: Utc
                .timestamp_opt(c_time.into(), c_time_nano)
                .single()
                .unwrap(),
            m_time: Utc
                .timestamp_opt(m_time.into(), m_time_nano)
                .single()
                .unwrap(),
            dev: 16777220,
            inode: 6187245,
            mode: "100644".to_owned(),
            uid: 501,
            gid: 20,
            size: 435,
            sha: "77fe5e0425e2f7ba652c54163bf283329456de39".to_owned(),
            name: "Cargo.toml".to_owned(),
        };
        let (input, result) = parse_git_index_entry(&entry).unwrap();
        assert_eq!(expected, result);
        assert_eq!(0, input.len());
    }

    #[test]
    fn can_parse_index() {
        let index = [
            68, 73, 82, 67, 0, 0, 0, 2, 0, 0, 0, 8, 99, 134, 133, 151, 26, 198, 1, 77, 99, 134,
            133, 151, 26, 198, 1, 77, 1, 0, 0, 4, 0, 94, 162, 84, 0, 0, 129, 164, 0, 0, 1, 245, 0,
            0, 0, 20, 0, 0, 1, 179, 119, 254, 94, 4, 37, 226, 247, 186, 101, 44, 84, 22, 59, 242,
            131, 50, 148, 86, 222, 57, 0, 10, 67, 97, 114, 103, 111, 46, 116, 111, 109, 108, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 55, 228, 89, 60, 218, 13,
            59, 253, 59, 115, 25, 117, 147, 194, 253, 192, 76, 197, 30, 0, 10, 115, 114, 99, 47,
            99, 108, 105, 46, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            248, 47, 249, 197, 177, 18, 227, 7, 129, 105, 212, 73, 244, 161, 101, 162, 57, 109,
            211, 250, 0, 15, 115, 114, 99, 47, 99, 111, 109, 109, 97, 110, 100, 115, 46, 114, 115,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 128, 130, 85, 252, 207, 120, 222,
            224, 14, 24, 245, 101, 253, 250, 193, 213, 243, 105, 43, 0, 12, 115, 114, 99, 47, 101,
            114, 114, 111, 114, 46, 114, 115, 0, 0, 0, 0, 0, 0, 99, 135, 164, 129, 40, 162, 231,
            102, 99, 135, 164, 129, 40, 162, 231, 102, 1, 0, 0, 4, 0, 95, 21, 64, 0, 0, 129, 164,
            0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 1, 242, 35, 27, 120, 172, 253, 99, 23, 237, 3, 87, 13,
            1, 115, 221, 32, 73, 54, 108, 72, 108, 0, 11, 115, 114, 99, 47, 109, 97, 105, 110, 46,
            114, 115, 0, 0, 0, 0, 0, 0, 0, 99, 135, 164, 129, 40, 172, 131, 113, 99, 135, 164, 129,
            40, 172, 131, 113, 1, 0, 0, 4, 0, 95, 21, 65, 0, 0, 129, 164, 0, 0, 1, 245, 0, 0, 0,
            20, 0, 0, 56, 238, 144, 94, 234, 103, 52, 105, 103, 149, 85, 165, 88, 40, 124, 88, 147,
            188, 98, 39, 214, 61, 0, 21, 115, 114, 99, 47, 111, 98, 106, 101, 99, 116, 95, 112, 97,
            114, 115, 101, 114, 115, 46, 114, 115, 0, 0, 0, 0, 0, 99, 118, 129, 163, 44, 243, 27,
            162, 99, 118, 129, 163, 44, 243, 27, 162, 1, 0, 0, 4, 0, 90, 191, 65, 0, 0, 129, 164,
            0, 0, 1, 245, 0, 0, 0, 20, 0, 0, 27, 81, 242, 62, 90, 226, 216, 80, 134, 183, 122, 28,
            135, 5, 147, 88, 112, 113, 51, 144, 147, 41, 0, 14, 115, 114, 99, 47, 111, 98, 106,
            101, 99, 116, 115, 46, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 129, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 94,
            245, 149, 200, 225, 77, 104, 173, 220, 22, 241, 0, 166, 218, 97, 147, 50, 113, 102, 91,
            0, 12, 115, 114, 99, 47, 117, 116, 105, 108, 115, 46, 114, 115, 0, 0, 0, 0, 0, 0, 84,
            82, 69, 69, 0, 0, 0, 53, 0, 56, 32, 49, 10, 94, 80, 75, 81, 83, 56, 251, 27, 118, 251,
            44, 61, 74, 48, 123, 44, 209, 219, 24, 88, 115, 114, 99, 0, 55, 32, 48, 10, 247, 119,
            37, 9, 236, 45, 66, 113, 190, 230, 234, 87, 91, 155, 125, 203, 198, 212, 185, 70, 164,
            227, 215, 175, 119, 29, 118, 67, 66, 89, 140, 127, 94, 30, 181, 10, 76, 188, 194, 142,
        ];

        let expected = Vec::from([
            "Cargo.toml",
            "src/cli.rs",
            "src/commands.rs",
            "src/error.rs",
            "src/main.rs",
            "src/object_parsers.rs",
            "src/objects.rs",
            "src/utils.rs",
        ]);

        let index = parse_git_index(&index).unwrap();
        let file_names: Vec<String> = index.entries.into_iter().map(|e| e.name).collect();
        assert_eq!(expected, file_names);
    }
}
