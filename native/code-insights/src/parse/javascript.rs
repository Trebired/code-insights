use tree_sitter::Node;

use super::{ancestor_of_kind, walk, Binding, Call, Callee, Context, FileAnalysis, ImportRef};

const CLASS_KINDS: &[&str] = &["class_declaration", "class", "abstract_class_declaration"];

fn import_bindings(node: Node, context: &Context) -> Vec<Binding> {
    let mut bindings = Vec::new();
    let Some(clause) = node.children(&mut node.walk()).find(|child| child.kind() == "import_clause") else {
        return bindings;
    };
    for child in clause.children(&mut clause.walk()) {
        match child.kind() {
            "identifier" => bindings.push(Binding { imported: "default".into(), local: context.text(child) }),
            "namespace_import" => {
                if let Some(name) = child.children(&mut child.walk()).find(|item| item.kind() == "identifier") {
                    bindings.push(Binding { imported: "*".into(), local: context.text(name) });
                }
            }
            "named_imports" => {
                for specifier in child.children(&mut child.walk()).filter(|item| item.kind() == "import_specifier") {
                    let imported = context.field_text(specifier, "name").unwrap_or_default();
                    let local = context.field_text(specifier, "alias").unwrap_or_else(|| imported.clone());
                    bindings.push(Binding { imported, local });
                }
            }
            _ => {}
        }
    }
    bindings
}

fn export_statement(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    if let Some(source) = node.child_by_field_name("source") {
        analysis.imports.push(ImportRef { bindings: Vec::new(), kind: "reexport", spec: context.string_value(source) });
        return;
    }
    let is_default = node.children(&mut node.walk()).any(|child| child.kind() == "default");
    if let Some(declaration) = node.child_by_field_name("declaration") {
        let name = context.field_text(declaration, "name").or_else(|| first_declarator_name(declaration, context));
        if let Some(name) = name {
            analysis.exports.insert(name.clone(), if is_default { "default".into() } else { name.clone() });
            if is_default {
                analysis.default_export = Some(name);
            }
        }
        return;
    }
    if is_default {
        if let Some(value) = node.child_by_field_name("value").filter(|value| value.kind() == "identifier") {
            analysis.default_export = Some(context.text(value));
        }
    }
    for clause in node.children(&mut node.walk()).filter(|child| child.kind() == "export_clause") {
        for specifier in clause.children(&mut clause.walk()).filter(|item| item.kind() == "export_specifier") {
            let local = context.field_text(specifier, "name").unwrap_or_default();
            let exported = context.field_text(specifier, "alias").unwrap_or_else(|| local.clone());
            if exported == "default" {
                analysis.default_export = Some(local.clone());
            }
            analysis.exports.insert(local, exported);
        }
    }
}

fn first_declarator_name(node: Node, context: &Context) -> Option<String> {
    node.children(&mut node.walk()).find(|child| child.kind() == "variable_declarator")
    .and_then(|declarator| context.field_text(declarator, "name"))
}

fn call_import(node: Node, context: &Context, analysis: &mut FileAnalysis) -> bool {
    let Some(function) = node.child_by_field_name("function") else { return false };
    let kind = match (function.kind(), context.text(function).as_str()) {
        ("import", _) => "dynamic",
        ("identifier", "require") => "require",
        _ => return false,
    };
    let argument = node.child_by_field_name("arguments")
    .and_then(|arguments| arguments.named_children(&mut arguments.walk()).next())
    .filter(|argument| argument.kind() == "string");
    if let Some(argument) = argument {
        analysis.imports.push(ImportRef { bindings: require_bindings(node, context), kind, spec: context.string_value(argument) });
    }
    true
}

fn require_bindings(node: Node, context: &Context) -> Vec<Binding> {
    let Some(declarator) = node.parent().filter(|parent| parent.kind() == "variable_declarator") else { return Vec::new() };
    let Some(name) = declarator.child_by_field_name("name") else { return Vec::new() };
    if name.kind() == "identifier" {
        return vec![Binding { imported: "*".into(), local: context.text(name) }];
    }
    let mut bindings = Vec::new();
    for child in name.named_children(&mut name.walk()) {
        if child.kind() == "shorthand_property_identifier_pattern" {
            bindings.push(Binding { imported: context.text(child), local: context.text(child) });
        }
    }
    bindings
}

fn owner_of(node: Node, context: &Context) -> Option<String> {
    ancestor_of_kind(node, CLASS_KINDS).and_then(|class| context.field_text(class, "name"))
}

fn definition(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name) = context.field_text(node, "name") {
                analysis.definitions.push(context.definition(node, name, "function", None, false));
            }
        }
        "method_definition" => {
            if let Some(name) = context.field_text(node, "name") {
                analysis.definitions.push(context.definition(node, name, "method", owner_of(node, context), false));
            }
        }
        "variable_declarator" | "public_field_definition" => {
            let value = node.child_by_field_name("value");
            let is_function = value.map_or(false, |value| matches!(value.kind(), "arrow_function" | "function_expression" | "function"));
            let name = context.field_text(node, "name").or_else(|| context.field_text(node, "property"));
            if let (true, Some(name)) = (is_function, name) {
                let owner = if node.kind() == "public_field_definition" { owner_of(node, context) } else { None };
                let kind = if owner.is_some() { "method" } else { "arrow" };
                analysis.definitions.push(context.definition(node, name, kind, owner, false));
            }
        }
        _ => {}
    }
}

fn call(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(function) = node.child_by_field_name("function") else { return };
    let callee = match function.kind() {
        "identifier" => Callee::Name(context.text(function)),
        "member_expression" => {
            let object = function.child_by_field_name("object");
            let property = context.field_text(function, "property").unwrap_or_default();
            match object.map(|object| (object.kind(), context.text(object))) {
                Some(("this", _)) => Callee::SelfMember(property),
                Some(("identifier", object)) => Callee::Member { object, property },
                _ => return,
            }
        }
        _ => return,
    };
    analysis.calls.push(Call { at: node.start_byte(), callee });
}

pub fn collect(root: Node, context: &Context, analysis: &mut FileAnalysis) {
    walk(root, |node| {
            match node.kind() {
                "import_statement" => {
                    if let Some(source) = node.child_by_field_name("source") {
                        let bindings = import_bindings(node, context);
                        analysis.imports.push(ImportRef { bindings, kind: "import", spec: context.string_value(source) });
                    }
                    return false;
                }
                "export_statement" => export_statement(node, context, analysis),
                "call_expression" => {
                    if !call_import(node, context, analysis) && context.functions {
                        call(node, context, analysis);
                    }
                }
                _ => {}
            }
            if context.functions {
                definition(node, context, analysis);
            }
            true
    });
    let exported = analysis.exports.clone();
    for definition in analysis.definitions.iter_mut().filter(|definition| definition.owner.is_none()) {
        definition.exported = exported.contains_key(&definition.name) || analysis.default_export.as_deref() == Some(&definition.name);
    }
}
