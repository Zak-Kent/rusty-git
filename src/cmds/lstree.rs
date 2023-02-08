use crate::utils;
use crate::objects::tree;

pub fn git_tree_leaf_to_string(tree::TreeLeaf { mode, path, sha }: &tree::TreeLeaf) -> String {
    let sha = utils::get_sha_from_binary(sha);
    format!("{mode} {sha} {path}\n")
}

pub fn git_tree_to_string(tree::Tree { contents }: tree::Tree) -> String {
    let mut output = String::new();
    for leaf in contents {
        output.push_str(&git_tree_leaf_to_string(&leaf));
    }
    output
}
