use deflate::write::ZlibEncoder;
use deflate::Compression;
use inflate::inflate_bytes_zlib;
use sha256;
use std::fs::{self, create_dir, metadata, read, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::from_utf8;

use crate::config as cfg;
use crate::error as err;
use crate::utils;

#[derive(Debug)]
pub enum GitObject {
    Commit,
    Tree,
    Blob(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
    pub gitconf: String,
}

fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

impl Repo {
    // new expects an existing git repo
    pub fn new(conf: cfg::Config) -> Result<Repo, err::Error> {
        let base_path = utils::git_repo_or_err(&conf.path)?;
        let gitdir = build_path(base_path.clone(), ".git")?;
        let gitconf_path = build_path(gitdir.clone(), "config")?;
        let gitconf = fs::read_to_string(gitconf_path)?;

        Ok(Repo {
            worktree: base_path,
            gitdir,
            gitconf,
        })
    }
}

pub fn find_gitdir_and_create_repo(conf: cfg::Config) -> Result<Repo, err::Error> {
    let mut path = conf.path;
    while !utils::is_git_repo(&path) {
        if let Some(p) = path.parent() {
            path = p.to_path_buf();
        } else {
            return Err(err::Error::GitNotARepo);
        }
    }

    let updated_conf = cfg::Config { path, ..conf };
    return Ok(Repo::new(updated_conf)?);
}

pub fn content_length(path: &Path) -> Result<u64, err::Error> {
    Ok(metadata(path)?.len())
}

pub fn path_exists(path: &Path) -> bool {
    metadata(path).is_ok()
}

fn git_obj_path_from_sha(sha: &str, repo: Repo) -> Result<PathBuf, err::Error> {
    let obj_path = repo
        .worktree
        .join(format!(".git/objects/{}/{}", &sha[..2], &sha[2..]));

    if path_exists(&obj_path) {
        return Ok(obj_path.to_path_buf());
    } else {
        return Err(err::Error::GitObjPathDoesntExist(
            obj_path.display().to_string(),
        ));
    }
}

pub fn read_object(sha: &str, repo: Repo) -> Result<String, err::Error> {
    let obj_path = git_obj_path_from_sha(sha, repo)?;
    let contents = read(&obj_path)?;

    let decoded = match inflate_bytes_zlib(&contents) {
        Ok(res) => res,
        Err(e) => return Err(err::Error::InflatingGitObj(e)),
    };

    // there's likely a better way to slice up the inflated Vec
    // avoiding the extra allocations for each part of the orig Vec
    let obj_type_b: Vec<u8> = decoded
        .iter()
        .take_while(|elm| **elm != b' ')
        .map(|b| *b)
        .collect();
    let obj_type = from_utf8(&obj_type_b)?;

    let obj_len_b: Vec<u8> = decoded
        .iter()
        .skip_while(|elm| **elm != b' ')
        .skip(1) // skip the space after obj type
        .take_while(|elm| **elm != b'\x00')
        .map(|b| *b)
        .collect();
    let obj_len = from_utf8(&obj_len_b)?.parse::<usize>()?;

    let obj_content_b: Vec<u8> = decoded
        .iter()
        .skip_while(|elm| **elm != b'\x00')
        .skip(1) // skip the null byte
        .map(|b| *b)
        .collect();
    let obj_content = from_utf8(&obj_content_b)?;

    if obj_len != obj_content.len() {
        return Err(err::Error::GitMalformedObject)
    }

    return Ok(obj_content.to_owned())
}

pub fn write_object(obj: GitObject, repo: Option<Repo>) -> Result<String, err::Error> {
    let path = match obj {
        GitObject::Blob(path) => path,
        _ => panic!("only implemented for Blobs!"),
    };

    let length = content_length(&path)?.to_string();
    let contents = fs::read(&path)?;
    let contents_with_header = [
        "blob".as_bytes(),
        " ".as_bytes(),
        length.as_bytes(),
        "\x00".as_bytes(),
        contents.as_slice(),
    ]
    .concat();

    let hash = sha256::digest_bytes(&contents_with_header);

    // The existance of a repo indicates that the contents of the obj should be
    // compressed and written to the appropriate dir/file in .git/objects
    if let Some(repo) = repo {
        let git_obj_dir = repo.worktree.join(format!(".git/objects/{}", &hash[..2]));
        let git_obj_path = git_obj_dir.join(format!("{}", &hash[2..]));

        if !path_exists(&git_obj_dir) {
            create_dir(&git_obj_dir)?;
        }

        if !path_exists(&git_obj_path) {
            let obj_file = File::create(&git_obj_path)?;
            let mut encoder = ZlibEncoder::new(obj_file, Compression::Default);
            encoder.write_all(&contents_with_header)?;
            encoder.finish()?;
        } else {
            println!("file with compressed contents already exists at that hash");
        }
    }
    return Ok(hash.to_owned());
}

#[cfg(test)]
mod object_tests {
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    use super::*;
    use crate::utils;

    #[test]
    fn git_repo_setup_test() {
        // unwrap will panic here if dir setup fails
        let worktree = utils::test_gitdir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join(".git/config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }

    #[test]
    fn repo_struct_creation_succeeds_when_in_git_repo() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();
        let cmd = utils::test_cmd("init", None);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        let _repo = Repo::new(config)?;
        Ok(())
    }

    #[test]
    fn repo_struct_creation_fails_when_not_in_git_repo() -> Result<(), err::Error> {
        let tmpdir = utils::test_tempdir().unwrap();
        let cmd = utils::test_cmd("add", None);
        let config = cfg::Config::new(cmd, Some(tmpdir.path().to_path_buf()))?;
        let repo = Repo::new(config);
        assert!(repo.is_err());
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_finds_parent_gitdir() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();

        // create a nested path with .git living a few levels above
        let nested_path = worktree.path().join("foo/bar/baz");
        create_dir_all(&nested_path)?;

        let cmd = utils::test_cmd("add", None);
        let config = cfg::Config::new(cmd, Some(nested_path))?;
        let repo = find_gitdir_and_create_repo(config)?;

        // check nested path was discarded when creating Repo.worktree
        assert_eq!(worktree.path(), repo.worktree);
        Ok(())
    }

    #[test]
    fn find_gitdir_and_create_repo_errors_when_no_gitdir_in_path() -> Result<(), err::Error> {
        let tmpdir = utils::test_tempdir().unwrap();
        let cmd = utils::test_cmd("add", None);
        let config = cfg::Config::new(cmd, Some(tmpdir.path().to_path_buf()))?;

        let repo = find_gitdir_and_create_repo(config);
        match repo {
            Err(err::Error::GitNotARepo) => assert!(true),
            _ => panic!("Repo creation should error!"),
        };
        Ok(())
    }

    #[test]
    fn generate_hash_and_write_compressed_file() -> Result<(), err::Error> {
        let worktree = utils::test_gitdir().unwrap();
        // TODO might want to reconsider the threading of args here wrt hash-object
        let cmd = utils::test_cmd("hash-object", None);
        let config = cfg::Config::new(cmd, Some(worktree.path().to_path_buf()))?;
        let repo = Repo::new(config)?;
        let repo_clone = repo.clone();

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let blob = GitObject::Blob(fp.to_owned());
        let hash = write_object(blob, Some(repo))?;

        assert_eq!(
            hash,
            "aa161e140ba95d5f611da742cedbdc98d11128a40d89a3c45b3a74f50f970897".to_owned()
        );

        let git_obj_path =
            worktree
                .path()
                .join(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]));
        assert_eq!(22, content_length(&git_obj_path)?);

        let obj_contents = read_object(&hash, repo_clone)?;
        assert_eq!("foobar\n", obj_contents);

        Ok(())
    }
}
