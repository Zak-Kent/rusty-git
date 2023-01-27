use nom::{
    bytes::complete::{is_not, tag, take, take_till1},
    character::complete::space1,
    multi::many1,
    IResult,
};
use std::fmt;
use std::str::from_utf8;

use super::{AsBytes, NameSha};
use crate::{cmd_mods::lstree, error as err, utils};

// a single entry in a Git tree obj file
type ParsedLeaf<'a> = (&'a [u8], &'a [u8], &'a [u8]);

pub fn parse_git_tree_leaf(input: &[u8]) -> IResult<&[u8], ParsedLeaf> {
    let (input, mode) = is_not(" ")(input)?;
    let (input, _) = space1(input)?;
    let (input, path) = take_till1(|c| c == b'\x00')(input)?;
    let (input, _) = tag(b"\x00")(input)?;
    let (input, bsha) = take(20usize)(input)?;
    return Ok((input, ParsedLeaf::from((mode, path, bsha))));
}

#[derive(Debug, PartialEq, Clone)]
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

impl AsBytes for TreeLeaf {
    fn as_bytes(&self) -> Vec<u8> {
        let file_info = [&self.mode, " ", &self.path, "\x00"]
            .map(|s| s.as_bytes())
            .concat();
        let mut leaf: Vec<u8> = Vec::new();
        leaf.extend_from_slice(&file_info);
        leaf.extend_from_slice(&self.sha);
        return leaf;
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tree {
    pub contents: Vec<TreeLeaf>,
}

impl AsBytes for Tree {
    fn as_bytes(&self) -> Vec<u8> {
        return self
            .contents
            .iter()
            .map(|e| e.as_bytes())
            .collect::<Vec<Vec<u8>>>()
            .concat();
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", lstree::git_tree_to_string(self.clone()))
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

#[cfg(test)]
mod tree_tests {
    use super::*;
    use sha1_smol as sha1;

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

        let round_trip_bytes = tree.as_bytes();
        assert_eq!(tree_file, round_trip_bytes);
    }
}
