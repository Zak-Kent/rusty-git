use std::fs::File;
use std::io::Write;

use crate::error as err;
use crate::objects as obj;
use crate::utils;
use crate::cmd_mods::refs;

pub fn list_all_tags(repo: &obj::Repo) -> Result<Vec<String>, err::Error> {
    let tags_path = repo.gitdir.join("refs/tags/");
    let tags = refs::gather_refs(Some(&tags_path), repo)?;
    Ok(tags)
}

pub fn create_lightweight_tag(
    tag_name: &String,
    object: &String,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    let tag_sha: String;
    if object == "HEAD" {
        tag_sha = utils::git_sha_from_head(repo)?;
    } else {
        tag_sha = object.to_owned();
    };
    let tag_path = repo.gitdir.join(format!("refs/tags/{}", tag_name));
    let mut tag = File::create(&tag_path)?;
    writeln!(tag, "{}", tag_sha)?;
    Ok(())
}


#[cfg(test)]
mod utils_tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn can_create_and_read_lightweight_tags() {
        let gitdir = test_utils::test_gitdir().unwrap();
        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();

        let tag_sha = "0e6cfc8b4209c9ecca33dbd30c41d1d4289736e1".to_owned();
        create_lightweight_tag(&"foo".to_owned(), &tag_sha, &repo).unwrap();

        let tag = list_all_tags(&repo).unwrap();
        let expected = format!("{tag_sha} refs/tags/foo\n");
        assert_eq!(&expected, tag.first().unwrap());
    }

}
