use tree_sitter::Node;

use super::{ancestor_of_kind, walk, Binding, Call, Callee, Context, FileAnalysis, ImportRef};

fn import_statement(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    for child in node.named_children(&mut node.walk()) {
        let (module, alias) = match child.kind() {
            "dotted_name" => (context.text(child), None),
            "aliased_import" => (context.field_text(child, "name").unwrap_or_default(), context.field_text(child, "alias")),
            _ => continue,
        };
        let local = alias.unwrap_or_else(|| module.clone());
        analysis.imports.push(ImportRef { bindings: vec![Binding { imported: "*".into(), local }], kind: "import", spec: module });
    }
}

fn import_from(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(module) = context.field_text(node, "module_name") else { return };
    let mut bindings = Vec::new();
    let mut cursor = node.walk();
    for child in node.children_by_field_name("name", &mut cursor) {
        let (imported, alias) = match child.kind() {
            "aliased_import" => (context.field_text(child, "name").unwrap_or_default(), context.field_text(child, "alias")),
            _ => (context.text(child), None),
        };
        let local = alias.unwrap_or_else(|| imported.clone());
        bindings.push(Binding { imported, local });
    }
    analysis.imports.push(ImportRef { bindings, kind: "from", spec: module });
}

fn call(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(function) = node.child_by_field_name("function") else { return };
    let callee = match function.kind() {
        "identifier" => Callee::Name(context.text(function)),
        "attribute" => {
            let object = function.child_by_field_name("object").map(|object| context.text(object)).unwrap_or_default();
            let property = context.field_text(function, "attribute").unwrap_or_default();
            if object == "self" || object == "cls" {
                Callee::SelfMember(property)
            } else {
                Callee::Member { object, property }
            }
        }
        _ => return,
    };
    analysis.calls.push(Call { at: node.start_byte(), callee });
}

pub fn collect(root: Node, context: &Context, analysis: &mut FileAnalysis) {
    walk(root, |node| {
            match node.kind() {
                "import_statement" => import_statement(node, context, analysis),
                "import_from_statement" => import_from(node, context, analysis),
                "call" if context.functions => call(node, context, analysis),
                "function_definition" if context.functions => {
                    if let Some(name) = context.field_text(node, "name") {
                        let owner = ancestor_of_kind(node, &["class_definition"]).and_then(|class| context.field_text(class, "name"));
                        let kind = if owner.is_some() { "method" } else { "function" };
                        let exported = owner.is_none() && !name.starts_with('_');
                        analysis.definitions.push(context.definition(node, name, kind, owner, exported));
                    }
                }
                _ => {}
            }
            true
    });
}
