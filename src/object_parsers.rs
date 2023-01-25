use chrono::{DateTime, TimeZone, Utc};
use nom::{
    bytes::complete::{is_a, is_not, take, take_till1},
    character::{
        complete::space1,
        is_newline,
    },
    error::{Error, ErrorKind},
    multi::many0,
    number::{
        complete::{u16, u32},
        Endianness::Big,
    },
    Err, IResult,
};
use sha1_smol as sha1;
use std::cmp::Ordering;
use std::str::from_utf8;

use crate::{error as err, utils};

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

pub fn parse_git_head(input: &[u8]) -> Result<String, err::Error> {
    let (input, _key) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (_, head_ref) = take_till1(is_newline)(input)?;
    return Ok(from_utf8(head_ref)?.to_owned());
}

pub trait NameSha {
    fn get_name_and_sha(&self, name_prefix: Option<String>) -> (String, String);
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

    #[test]
    fn can_parse_git_head() {
        let head_file = "ref: refs/heads/main".as_bytes();
        assert_eq!("refs/heads/main", parse_git_head(head_file).unwrap());
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
}
