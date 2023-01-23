use crate::object_parsers as objp;

pub fn git_tree_leaf_to_string(objp::TreeLeaf { mode, path, sha }: &objp::TreeLeaf) -> String {
    return format!("{mode} {sha} {path}\n");
}

pub fn git_tree_to_string(objp::Tree { contents }: objp::Tree) -> String {
    let mut output = String::new();
    for leaf in contents {
        output.push_str(&git_tree_leaf_to_string(&leaf));
    }
    return output;
}
