use deflate::write::ZlibEncoder;
use deflate::Compression;
use inflate::inflate_bytes_zlib;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till1},
    character::{complete::space1, is_newline},
    error::{Error, ErrorKind},
    Err, IResult,
};
use sha1_smol as sha1;
use std::fs::{self as fs, create_dir, read, File};
use std::io::Write;
use std::str::from_utf8;
use std::path::PathBuf;

use crate::error as err;
// use crate::objects as obj;
use crate::utils;

pub mod blob;
pub mod commit;
pub mod tree;

#[derive(Debug, Clone)]
pub struct Repo {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
    pub gitconf: String,
}

impl Repo {
    // new expects an existing git repo
    pub fn new(path: PathBuf) -> Result<Repo, err::Error> {
        let base_path = utils::git_repo_or_err(&PathBuf::from(path))?;
        let gitdir = utils::build_path(base_path.clone(), ".git")?;
        let gitconf_path = utils::build_path(gitdir.clone(), "config")?;
        let gitconf = fs::read_to_string(gitconf_path)?;

        Ok(Repo {
            worktree: base_path,
            gitdir,
            gitconf,
        })
    }
}

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
    Commit(commit::Commit),
}

pub fn parse_git_obj<'a>(input: &'a [u8], sha: &'a str) -> Result<GitObj, err::Error> {
    let (input, obj) = alt((tag("blob"), tag("commit"), tag("tree")))(input)?;
    let (contents, len) = parse_obj_len(input)?;
    if len != contents.len() {
        return Err(err::Error::GitMalformedObject);
    }
    match obj {
        b"blob" => Ok(GitObj::Blob(blob::Blob::new(contents))),
        b"tree" => Ok(GitObj::Tree(tree::parse_git_tree(contents)?)),
        b"commit" => Ok(GitObj::Commit(commit::parse_kv_list_msg(contents, sha)?)),
        _ => Err(err::Error::GitUnrecognizedObjInHeader(from_utf8(&obj)?.to_string())),
    }
}

pub fn read_object(sha: &str, repo: &Repo) -> Result<GitObj, err::Error> {
    let obj_path = utils::git_obj_path_from_sha(sha, &repo)?;
    let contents = read(&obj_path)?;
    let decoded = match inflate_bytes_zlib(&contents) {
        Ok(res) => res,
        Err(e) => return Err(err::Error::InflatingGitObj(e)),
    };
    return Ok(parse_git_obj(&decoded, &sha)?);
}

pub fn read_object_as_string(sha: &str, repo: &Repo) -> Result<String, err::Error> {
    let gitobject = read_object(sha, &repo)?;
    match gitobject {
        GitObj::Blob(blob) => Ok(format!("{}", blob)),
        GitObj::Tree(tree) => Ok(format!("{}", tree)),
        GitObj::Commit(commit) => Ok(format!("{}", commit)),
    }
}

pub fn write_object(
    obj: GitObj,
    repo: Option<&Repo>,
) -> Result<sha1::Digest, err::Error> {
    let obj_bytes = match obj {
        GitObj::Blob(blob) => blob.as_bytes(),
        GitObj::Tree(tree) => tree.as_bytes(),
        GitObj::Commit(commit) => commit.as_bytes(),
    };

    let mut hasher = sha1::Sha1::new();
    hasher.update(&obj_bytes);
    let digest = hasher.digest();

    // The existance of a repo indicates that the contents of the file should be
    // compressed and written to the appropriate dir/file in .git/objects
    if let Some(repo) = repo {
        utils::git_check_for_rusty_git_allowed(repo)?;
        let hash = digest.to_string();
        let git_obj_dir = repo.worktree.join(format!(".git/objects/{}", &hash[..2]));
        let git_obj_path = git_obj_dir.join(format!("{}", &hash[2..]));

        if !git_obj_dir.exists() {
            create_dir(&git_obj_dir)?;
        }

        if !git_obj_path.exists() {
            let obj_file = File::create(&git_obj_path)?;
            let mut encoder = ZlibEncoder::new(obj_file, Compression::Default);
            encoder.write_all(&obj_bytes)?;
            encoder.finish()?;
        } else {
            println!("file with compressed contents already exists at that hash");
        }
    }
    return Ok(digest);
}

#[cfg(test)]
mod object_mod_tests {
    use super::*;
    use crate::test_utils;
    use std::fs;

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

    #[test]
    fn generate_hash_and_write_compressed_file() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();
        let repo = Repo::new(worktree.path().to_path_buf())?;

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let blob = blob::blob_from_path(fp)?;
        let sha = write_object(blob, Some(&repo))?.to_string();

        assert_eq!(sha, "323fae03f4606ea9991df8befbb2fca795e648fa".to_owned());

        let git_obj_path =
            worktree
                .path()
                .join(format!(".git/objects/{}/{}", &sha[..2], &sha[2..]));
        assert_eq!(22, test_utils::content_length(&git_obj_path)?);

        let obj_contents = read_object_as_string(&sha, &repo)?;
        assert_eq!("foobar\n", obj_contents);

        Ok(())
    }

    fn find_gitdir_and_create_repo(path: String) -> Result<Repo, err::Error> {
        let mut path = PathBuf::from(path);

        while !utils::is_git_repo(&path) {
            if let Some(p) = path.parent() {
                path = p.to_path_buf();
            } else {
                return Err(err::Error::GitNotARepo);
            }
        }

        return Ok(Repo::new(path)?);
    }

    #[test]
    fn git_repo_setup_test() {
        // unwrap will panic here if dir setup fails
        let worktree = test_utils::test_gitdir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join(".git/config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }

    #[test]
    fn repo_struct_creation_succeeds_when_in_git_repo() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();
        let _repo = Repo::new(worktree.path().to_path_buf())?;
        Ok(())
    }

    #[test]
    fn repo_struct_creation_fails_when_not_in_git_repo() -> Result<(), err::Error> {
        let tmpdir = test_utils::test_tempdir().unwrap();
        let repo = Repo::new(tmpdir.path().to_path_buf());
        assert!(repo.is_err());
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_finds_parent_gitdir() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();

        // create a nested path with .git living a few levels above
        let nested_path = worktree.path().join("foo/bar/baz");
        fs::create_dir_all(&nested_path)?;

        let repo = find_gitdir_and_create_repo(nested_path.to_str().unwrap().to_owned())?;

        // check nested path was discarded when creating Repo.worktree
        assert_eq!(worktree.path(), repo.worktree);
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_errors_when_no_gitdir_in_path() -> Result<(), err::Error> {
        let tmpdir = test_utils::test_tempdir().unwrap();

        let repo = find_gitdir_and_create_repo(tmpdir.path().to_str().unwrap().to_owned());
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }
}
