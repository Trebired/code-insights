use std::collections::{BTreeMap, HashMap};

use crate::git::TreeEntry;
use crate::output::{TreeNode, TreeOutput};

struct Builder {
    children: BTreeMap<String, Builder>,
    node: Option<TreeNode>,
}

impl Builder {
    fn new() -> Self {
        Builder { children: BTreeMap::new(), node: None }
    }
}

fn directory_node(name: &str, path: &str) -> TreeNode {
    TreeNode {
        kind: "dir", language: None, mode: "040000".to_string(), name: name.to_string(),
        object: String::new(), path: path.to_string(), size: None, children: None,
    }
}

fn insert(root: &mut Builder, entry: &TreeEntry, languages: &HashMap<String, String>) {
    let parts: Vec<&str> = entry.path.split('/').filter(|part| !part.is_empty()).collect();
    let mut cursor = root;
    let mut path = String::new();
    for (index, part) in parts.iter().enumerate() {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(part);
        let last = index == parts.len() - 1;
        cursor = cursor.children.entry(part.to_string()).or_insert_with(Builder::new);
        if cursor.node.is_none() || (last && entry.kind == "tree") {
            let mut node = directory_node(part, &path);
            if last {
                node.mode = entry.mode.clone();
                node.object = entry.object.clone();
                if entry.kind != "tree" {
                    node.kind = "file";
                    node.size = entry.size;
                    node.language = languages.get(&entry.path).cloned();
                }
            }
            cursor.node = Some(node);
        }
    }
}

fn sort_key(node: &TreeNode) -> (u8, String, String) {
    (if node.kind == "dir" { 0 } else { 1 }, node.name.to_lowercase(), node.name.clone())
}

fn finish(builder: Builder) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = builder.children.into_values().filter_map(|child| {
            let mut node = child.node.clone()?;
            if node.kind == "dir" {
                node.children = Some(finish(child));
            }
            Some(node)
    }).collect();
    nodes.sort_by_key(sort_key);
    nodes
}

fn format_ascii(nodes: &[TreeNode], prefix: &str, out: &mut Vec<String>) {
    for (index, node) in nodes.iter().enumerate() {
        let last = index == nodes.len() - 1;
        out.push(format!("{}{}{}", prefix, if last { "└── " } else { "├── " }, node.name));
        if let Some(children) = node.children.as_ref().filter(|items| !items.is_empty()) {
            format_ascii(children, &format!("{}{}", prefix, if last { "    " } else { "│   " }), out);
        }
    }
}

pub fn build(entries: &[TreeEntry], languages: &HashMap<String, String>) -> TreeOutput {
    let mut root = Builder::new();
    for entry in entries {
        insert(&mut root, entry, languages);
    }
    let nested = finish(root);
    let mut lines = Vec::new();
    format_ascii(&nested, "", &mut lines);
    TreeOutput { ascii: lines.join("\n"), nested }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: &str, path: &str) -> TreeEntry {
        TreeEntry { kind: kind.to_string(), mode: "100644".to_string(), object: "x".to_string(), path: path.to_string(), size: Some(1) }
    }

    #[test]
    fn nests_dirs_first_and_formats_ascii() {
        let entries = vec![
            entry("blob", "README.md"),
            entry("tree", "src"),
            entry("blob", "src/b.ts"),
            entry("blob", "src/a.ts"),
            entry("blob", "LICENSE")
        ];
        let tree = build(&entries, &HashMap::new());
        assert_eq!(tree.ascii, "├── src\n│   ├── a.ts\n│   └── b.ts\n├── LICENSE\n└── README.md");
        assert_eq!(tree.nested[0].children.as_ref().unwrap().len(), 2);
    }
}
