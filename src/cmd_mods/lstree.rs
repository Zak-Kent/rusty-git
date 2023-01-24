use crate::{object_parsers as objp, utils};

pub fn git_tree_leaf_to_string(objp::TreeLeaf { mode, path, sha }: &objp::TreeLeaf) -> String {
    let sha = utils::get_sha_from_binary(sha);
    return format!("{mode} {sha} {path}\n");
}

pub fn git_tree_to_string(objp::Tree { contents }: objp::Tree) -> String {
    let mut output = String::new();
    for leaf in contents {
        output.push_str(&git_tree_leaf_to_string(&leaf));
    }
    return output;
}
