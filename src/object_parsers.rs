use chrono::{DateTime, TimeZone, Utc};
use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take, take_till1, take_while1},
    character::{
        complete::{space0, space1},
        is_newline,
    },
    error::{Error, ErrorKind},
    multi::{many0, many1},
    number::{
        complete::{u16, u32},
        Endianness::Big,
    },
    Err, IResult,
};
use sha1_smol as sha1;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::from_utf8;

use crate::objects as obj;
use crate::{error as err, utils};

// TODO: figure out a way to make nom errors more specific
fn generic_nom_failure(input: &[u8]) -> Err<Error<&[u8]>> {
    Err::Failure(Error {
        input,
        code: ErrorKind::Fail,
    })
}

fn nom_many0_err(input: &[u8]) -> Err<Error<&[u8]>> {
    // this error type allows the parser to continue with the input
    // after the failed parse, which is needed when the entries in
    // the index file have been exahusted but extension info and sha
    // remains
    Err::Error(Error {
        input,
        code: ErrorKind::Many0,
    })
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

pub fn parse_git_obj<'a>(
    input: &'a [u8],
    path: &'a PathBuf,
    sha: &'a str,
) -> Result<obj::GitObject, err::Error> {
    let (input, obj) = parse_obj_type(input)?;
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

impl ToBinary for KvsMsg {
    fn to_binary(&self) -> Vec<u8> {
        let mut commit_bytes: Vec<u8> = Vec::new();
        for elm in self.kvs_order.clone() {
            if let Some((k, v)) = self.kvs.get_key_value(&elm) {
                let mut line: Vec<u8> = [k.clone(), [b' '].to_vec(), v.clone(), [b'\n'].to_vec()]
                    .into_iter()
                    .flatten()
                    .collect();
                commit_bytes.append(&mut line);
            }
        }
        commit_bytes.push(b'\n');
        commit_bytes.append(&mut self.msg.clone());

        let mut output_bytes: Vec<u8> = [
            b"commit".to_vec(),
            [b' '].to_vec(),
            commit_bytes.len().to_string().as_bytes().to_vec(),
            [b'\x00'].to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        output_bytes.append(&mut commit_bytes);
        return output_bytes;
    }
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

// a single entry in a GitObjType::Tree file
type ParsedLeaf<'a> = (&'a [u8], &'a [u8], &'a [u8]);

pub fn parse_git_tree_leaf(input: &[u8]) -> IResult<&[u8], ParsedLeaf> {
    let (input, mode) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (input, path) = take_till1(|c| c == b'\x00')(input)?;
    let (input, _) = tag(b"\x00")(input)?;
    let (input, bsha) = take(20usize)(input)?;
    return Ok((input, ParsedLeaf::from((mode, path, bsha))));
}

pub trait NameSha {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String);
}

#[derive(Debug, PartialEq)]
pub struct TreeLeaf {
    pub mode: String,
    pub path: String,
    pub sha: Vec<u8>,
}

impl NameSha for TreeLeaf {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String) {
        let sha = utils::get_sha_from_binary(&self.sha);
        if let Some(prefix) = name_prefix {
            return (format!("{prefix}/{}", self.path), sha);
        } else {
            return (self.path.clone(), sha);
        }
    }
}

impl ToBinary for TreeLeaf {
    fn to_binary(&self) -> Vec<u8> {
        let file_info = [&self.mode, " ", &self.path, "\x00"]
            .map(|s| s.as_bytes())
            .concat();
        let mut leaf: Vec<u8> = Vec::new();
        leaf.extend_from_slice(&file_info);
        leaf.extend_from_slice(&self.sha);
        return leaf;
    }
}

#[derive(Debug, PartialEq)]
pub struct Tree {
    pub contents: Vec<TreeLeaf>,
}

impl ToBinary for Tree {
    fn to_binary(&self) -> Vec<u8> {
        return self
            .contents
            .iter()
            .map(|e| e.to_binary())
            .collect::<Vec<Vec<u8>>>()
            .concat();
    }
}

pub fn parse_git_tree(input: &[u8]) -> Result<Tree, err::Error> {
    let (_, leaves) = many1(parse_git_tree_leaf)(input)?;
    let mut contents: Vec<TreeLeaf> = Vec::new();

    for (mode, path, sha) in leaves {
        contents.push(TreeLeaf {
            mode: from_utf8(mode)?.to_owned(),
            path: from_utf8(path)?.to_owned(),
            sha: sha.to_vec(),
        })
    }

    return Ok(Tree { contents });
}

// ------------- git index file parsers -----------------

pub trait ToBinary {
    fn to_binary(&self) -> Vec<u8>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IndexEntry {
    pub c_time: DateTime<Utc>,
    pub m_time: DateTime<Utc>,
    pub dev: u32,
    pub inode: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub sha: Vec<u8>,
    pub name: String,
}

impl Ord for IndexEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for IndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NameSha for IndexEntry {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String) {
        let sha = utils::get_sha_from_binary(&self.sha);
        if let Some(prefix) = name_prefix {
            return (format!("{prefix}/{}", self.name), sha);
        } else {
            return (self.name.clone(), sha);
        }
    }
}

impl ToBinary for IndexEntry {
    fn to_binary(&self) -> Vec<u8> {
        let c_seconds = self.c_time.timestamp() as u32;
        let c_nanos = self.c_time.timestamp_subsec_nanos();
        let m_seconds = self.m_time.timestamp() as u32;
        let m_nanos = self.m_time.timestamp_subsec_nanos();

        let index_meta_info: Vec<u8> = [
            c_seconds, c_nanos, m_seconds, m_nanos, self.dev, self.inode, self.mode, self.uid,
            self.gid, self.size,
        ]
        .iter()
        .flat_map(|i| i.to_be_bytes())
        .collect();

        let name_size = self.name.len() as u16;
        let entry_length = 62 + name_size;
        let padding_bytes: Vec<u8> = (0..(8 - entry_length % 8)).map(|_| b'\0').collect();

        return [
            index_meta_info,
            self.sha.clone(),
            name_size.to_be_bytes().to_vec(),
            self.name.as_bytes().to_vec(),
            padding_bytes,
        ]
        .concat();
    }
}

pub fn parse_git_index_entry(input: &[u8]) -> IResult<&[u8], IndexEntry> {
    let (input, c_time) = u32(Big)(input)?;
    let (input, c_time_nano) = u32(Big)(input)?;
    let c_time_dt;
    if let Some(ct) = Utc.timestamp_opt(c_time.into(), c_time_nano).single() {
        c_time_dt = ct;
    } else {
        return Err(nom_many0_err(input));
    };

    let (input, m_time) = u32(Big)(input)?;
    let (input, m_time_nano) = u32(Big)(input)?;
    let m_time_dt;
    if let Some(mt) = Utc.timestamp_opt(m_time.into(), m_time_nano).single() {
        m_time_dt = mt;
    } else {
        return Err(nom_many0_err(input));
    };

    let (input, dev) = u32(Big)(input)?;
    let (input, inode) = u32(Big)(input)?;

    let (input, mode) = u32(Big)(input)?;
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
        return Err(nom_many0_err(input));
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
            mode,
            uid,
            gid,
            size,
            sha: bsha.to_vec(),
            name: parsed_name.to_owned(),
        },
    ));
}

#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
    pub extensions: Vec<u8>,
}

impl Index {
    pub fn new(entry: IndexEntry) -> Result<Index, err::Error> {
        return Ok(Index {
            entries: [entry].to_vec(),
            extensions: [].to_vec(),
        });
    }
}

impl ToBinary for Index {
    fn to_binary(&self) -> Vec<u8> {
        let header = [
            "DIRC".as_bytes(),
            &[0x00, 0x00, 0x00, 0x02].to_vec(),
            &(self.entries.len() as u32).to_be_bytes(),
        ]
        .concat();

        let entries: Vec<u8> = self
            .entries
            .iter()
            .map(|i| i.to_binary())
            .collect::<Vec<Vec<u8>>>()
            .concat();

        let index_contents = [header, entries, self.extensions.clone()].concat();

        let mut hasher = sha1::Sha1::new();
        hasher.update(&index_contents);
        let hash = hasher.digest().bytes();

        return [index_contents, hash.to_vec()].concat();
    }
}

pub fn parse_git_index(input: &[u8]) -> Result<Index, err::Error> {
    let (input, _dirc) = is_a("DIRC")(input)?;
    let (input, version) = u32(Big)(input)?;
    if version != 2 {
        return Err(err::Error::GitUnrecognizedIndexVersion(version));
    }
    let (input, _num_entries) = u32(Big)(input)?;
    let (input, entries) = many0(parse_git_index_entry)(input)?;

    // need to drop the 20 byte index contents hash
    let ext_len = input.len() - 20;
    let extensions = input[..ext_len].to_vec();

    return Ok(Index {
        entries,
        extensions,
    });
}

#[cfg(test)]
mod object_parsing_tests {
    use super::*;
    use crate::test_utils;
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

    fn get_sha_bytes(file_name: &str) -> Vec<u8> {
        let mut hasher = sha1::Sha1::new();
        hasher.update(file_name.as_bytes());
        let sha = hasher.digest().to_string();
        return hex::decode(sha).unwrap();
    }

    fn make_git_tree_leaf(file_name: &str, perms: &str) -> Vec<u8> {
        let file_info = [perms, " ", file_name, "\x00"]
            .map(|s| s.as_bytes())
            .concat();
        let bsha = get_sha_bytes(file_name);

        let mut leaf: Vec<u8> = Vec::new();
        leaf.extend_from_slice(&file_info);
        leaf.extend_from_slice(&bsha);
        return leaf;
    }

    #[test]
    fn can_parse_git_tree_leaf() {
        let file_path = "src/foo.txt";
        let leaf = make_git_tree_leaf(file_path, "100644");
        let bsha = get_sha_bytes(file_path);
        let expected_val = ParsedLeaf::from((b"100644", file_path.as_bytes(), &bsha));
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
                    sha: get_sha_bytes("src/foo.txt"),
                },
                TreeLeaf {
                    mode: "040000".to_owned(),
                    path: "tests".to_owned(),
                    sha: get_sha_bytes("tests"),
                },
                TreeLeaf {
                    mode: "100644".to_owned(),
                    path: "src/bar.txt".to_owned(),
                    sha: get_sha_bytes("src/bar.txt"),
                },
            ]),
        };
        let tree = parse_git_tree(&tree_file).unwrap();
        assert_eq!(expected_val, tree);

        let round_trip_bytes = tree.to_binary();
        assert_eq!(tree_file, round_trip_bytes);
    }

    #[test]
    fn can_parse_index_entry() {
        let entry = test_utils::fake_index_entry();

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
            mode: 33188,
            uid: 501,
            gid: 20,
            size: 435,
            sha: [
                119, 254, 94, 4, 37, 226, 247, 186, 101, 44, 84, 22, 59, 242, 131, 50, 148, 86,
                222, 57,
            ]
            .to_vec(),
            name: "Cargo.toml".to_owned(),
        };
        let (input, result) = parse_git_index_entry(&entry).unwrap();
        assert_eq!(expected, result);
        assert_eq!(0, input.len());

        let round_trip_bytes = result.to_binary();
        assert_eq!(entry.to_vec(), round_trip_bytes);
    }

    #[test]
    fn can_parse_index() {
        let index = test_utils::fake_index_bytes();

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

        let parsed_index = parse_git_index(&index).unwrap();
        let parsed_index_clone = parsed_index.clone();
        let file_names: Vec<String> = parsed_index.entries.into_iter().map(|e| e.name).collect();
        assert_eq!(expected, file_names);

        let round_trip_bytes = parsed_index_clone.to_binary();
        assert_eq!(index.to_vec(), round_trip_bytes);
    }

    #[test]
    fn can_parse_index_with_no_entries() {
        // an index with no entries can happen if someone adds a file using
        // 'git add <file>' and then removes it with 'git rm --cached <file>'
        // this test more importantly checks that a failed parse of an entry
        // doesn't error out of parsing all together. Sometimes the entry
        // parser might attempt to parse the sha at the end of the index as
        // an entry and that should fail but allow parsing to continue with
        // the next parser after the index_entry_parser
        let index = test_utils::fake_index_no_entry();
        let parsed_index = parse_git_index(&index).unwrap();
        let expected = Index {
            entries: [].to_vec(),
            extensions: [].to_vec(),
        };
        assert_eq!(expected, parsed_index);
    }

    #[test]
    fn can_round_trip_commit() {
        let commit_bytes = test_utils::fake_commit();
        let sha = "8f30e364422bba93030062297731f00a1510984b";
        let parsed_commit = parse_git_obj(&commit_bytes, &PathBuf::from("foo"), sha).unwrap();

        // instances of the KvsMsg struct are the in mem representation of commits
        // it might make sense to combine KvsMsg with GitObject at some point.
        let parsed_kvsmsg = parse_kv_list_msg(&parsed_commit.contents, sha).unwrap();
        let round_trip_commit = parsed_kvsmsg.to_binary();
        assert_eq!(commit_bytes, round_trip_commit);
    }
}
