use deflate::write::ZlibEncoder;
use deflate::Compression;
use sha1_smol as sha1;
use std::fs::{self, create_dir, read, File};
use std::io::Write;
use std::path::PathBuf;

use crate::error as err;
use crate::utils;

#[derive(Debug, PartialEq)]
pub enum GitObjTyp {
    Commit,
    Tree,
    Blob,
}

// a file in the .git/objects dir
#[derive(Debug)]
pub struct GitObject {
    pub obj: GitObjTyp,
    pub len: usize,
    pub contents: Vec<u8>,
    pub source: PathBuf,
    pub sha: String,
}

// a file that is being fed into git
#[derive(Debug)]
pub struct SourceFile {
    pub typ: GitObjTyp,
    pub source: PathBuf,
}

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

#[allow(dead_code)]
pub fn find_gitdir_and_create_repo(path: String) -> Result<Repo, err::Error> {
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

pub fn write_object(src: SourceFile, repo: Option<&Repo>) -> Result<sha1::Digest, err::Error> {
    let path = match src {
        SourceFile {
            typ: GitObjTyp::Blob,
            source,
            ..
        } => source,
        _ => panic!("only implemented for Blobs!"),
    };

    let length = utils::content_length(&path)?.to_string();
    let contents = read(&path)?;
    let contents_with_header = [
        "blob".as_bytes(),
        " ".as_bytes(),
        length.as_bytes(),
        "\x00".as_bytes(),
        contents.as_slice(),
    ]
    .concat();

    let mut hasher = sha1::Sha1::new();
    hasher.update(&contents_with_header);
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
            encoder.write_all(&contents_with_header)?;
            encoder.finish()?;
        } else {
            println!("file with compressed contents already exists at that hash");
        }
    }
    return Ok(digest);
}

#[cfg(test)]
mod object_tests {
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    use super::*;
    use crate::test_utils;
    use crate::utils;
    use crate::object_mods as objm;

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
        create_dir_all(&nested_path)?;

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

    #[test]
    fn generate_hash_and_write_compressed_file() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();
        let repo = Repo::new(worktree.path().to_path_buf())?;

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let src = SourceFile {
            typ: GitObjTyp::Blob,
            source: fp.to_owned(),
        };
        let hash = write_object(src, Some(&repo))?.to_string();

        assert_eq!(hash, "323fae03f4606ea9991df8befbb2fca795e648fa".to_owned());

        let git_obj_path =
            worktree
                .path()
                .join(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]));
        assert_eq!(22, utils::content_length(&git_obj_path)?);

        let obj_contents = objm::read_object_as_string(&hash, &repo)?;
        assert_eq!("foobar\n", obj_contents);

        Ok(())
    }
}
