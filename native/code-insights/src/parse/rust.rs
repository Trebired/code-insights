use tree_sitter::Node;

use super::{ancestor_of_kind, walk, Binding, Call, Callee, Context, FileAnalysis, ImportRef};

fn join(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() { segment.to_string() } else { format!("{}::{}", prefix, segment) }
}

fn flatten_use(node: Node, prefix: &str, context: &Context, out: &mut Vec<(String, String)>) {
    match node.kind() {
        "identifier" | "self" | "crate" | "super" => {
            let segment = context.text(node);
            let local = if segment == "self" { prefix.rsplit("::").next().unwrap_or(prefix).to_string() } else { segment.clone() };
            let path = if segment == "self" { prefix.to_string() } else { join(prefix, &segment) };
            out.push((path, local));
        }
        "scoped_identifier" => {
            let path = join(prefix, &context.text(node));
            let local = context.field_text(node, "name").unwrap_or_default();
            out.push((path, local));
        }
        "use_as_clause" => {
            if let (Some(path), Some(alias)) = (node.child_by_field_name("path"), context.field_text(node, "alias")) {
                out.push((join(prefix, &context.text(path)), alias));
            }
        }
        "scoped_use_list" => {
            let base = node.child_by_field_name("path").map(|path| join(prefix, &context.text(path))).unwrap_or_else(|| prefix.to_string());
            if let Some(list) = node.child_by_field_name("list") {
                flatten_use(list, &base, context, out);
            }
        }
        "use_list" => {
            for child in node.named_children(&mut node.walk()) {
                flatten_use(child, prefix, context, out);
            }
        }
        "use_wildcard" => {
            let base = node.named_child(0).map(|path| join(prefix, &context.text(path))).unwrap_or_else(|| prefix.to_string());
            out.push((base, "*".to_string()));
        }
        _ => {}
    }
}

fn use_declaration(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(argument) = node.child_by_field_name("argument") else { return };
    let mut paths = Vec::new();
    flatten_use(argument, "", context, &mut paths);
    for (path, local) in paths {
        let imported = path.rsplit("::").next().unwrap_or(&path).to_string();
        analysis.imports.push(ImportRef { bindings: vec![Binding { imported, local }], kind: "use", spec: path });
    }
}

fn impl_owner(node: Node, context: &Context) -> Option<String> {
    let container = ancestor_of_kind(node, &["impl_item", "trait_item"])?;
    let owner = context.field_text(container, if container.kind() == "impl_item" { "type" } else { "name" })?;
    Some(owner.split('<').next().unwrap_or(&owner).trim().to_string())
}

fn call(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(function) = node.child_by_field_name("function") else { return };
    let callee = match function.kind() {
        "identifier" => Callee::Name(context.text(function)),
        "scoped_identifier" => {
            let text = context.text(function);
            Callee::Path(text.split("::").map(|segment| segment.split('<').next().unwrap_or(segment).to_string()).collect())
        }
        "field_expression" => {
            let value = function.child_by_field_name("value").map(|value| context.text(value)).unwrap_or_default();
            let field = context.field_text(function, "field").unwrap_or_default();
            if value == "self" { Callee::SelfMember(field) } else { return }
        }
        _ => return,
    };
    analysis.calls.push(Call { at: node.start_byte(), callee });
}

fn path_attribute(node: Node, context: &Context) -> Option<String> {
    let mut sibling = node.prev_named_sibling();
    while let Some(current) = sibling.filter(|item| item.kind() == "attribute_item") {
        let text = context.text(current);
        if let Some(rest) = text.split_once("path").map(|(_, rest)| rest) {
            if let Some(value) = rest.split('"').nth(1) {
                return Some(value.to_string());
            }
        }
        sibling = current.prev_named_sibling();
    }
    None
}

fn include_macro(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let name = context.field_text(node, "macro").unwrap_or_default();
    if name != "include" && name != "include_str" {
        return;
    }
    let text = context.text(node);
    if let Some(value) = text.split('"').nth(1) {
        analysis.imports.push(ImportRef { bindings: Vec::new(), kind: "include", spec: value.to_string() });
    }
}

pub fn collect(root: Node, context: &Context, analysis: &mut FileAnalysis) {
    walk(root, |node| {
            match node.kind() {
                "mod_item" if node.child_by_field_name("body").is_none() => {
                    if let Some(path) = path_attribute(node, context) {
                        analysis.imports.push(ImportRef { bindings: Vec::new(), kind: "include", spec: path });
                    } else if let Some(name) = context.field_text(node, "name") {
                        analysis.imports.push(ImportRef { bindings: Vec::new(), kind: "mod", spec: name });
                    }
                }
                "macro_invocation" => include_macro(node, context, analysis),
                "use_declaration" => {
                    use_declaration(node, context, analysis);
                    return false;
                }
                "call_expression" if context.functions => call(node, context, analysis),
                "function_item" if context.functions => {
                    if let Some(name) = context.field_text(node, "name") {
                        let owner = impl_owner(node, context);
                        let kind = if owner.is_some() { "method" } else { "function" };
                        let exported = node.children(&mut node.walk()).any(|child| child.kind() == "visibility_modifier");
                        analysis.definitions.push(context.definition(node, name, kind, owner, exported));
                    }
                }
                _ => {}
            }
            true
    });
}
