use tree_sitter::Node;

use super::{walk, Binding, Call, Callee, Context, FileAnalysis, ImportRef};

fn import_spec(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(path) = node.child_by_field_name("path").map(|path| context.string_value(path)) else { return };
    let local = context.field_text(node, "name").unwrap_or_else(|| path.rsplit('/').next().unwrap_or(&path).to_string());
    analysis.imports.push(ImportRef { bindings: vec![Binding { imported: "*".into(), local }], kind: "import", spec: path });
}

fn receiver(node: Node, context: &Context) -> (Option<String>, Option<String>) {
    let Some(parameters) = node.child_by_field_name("receiver") else { return (None, None) };
    let Some(parameter) = parameters.named_children(&mut parameters.walk()).find(|child| child.kind() == "parameter_declaration") else {
        return (None, None);
    };
    let variable = context.field_text(parameter, "name");
    let owner = context.field_text(parameter, "type").map(|kind| kind.trim_start_matches('*').split('[').next().unwrap_or("").to_string());
    (owner, variable)
}

fn call(node: Node, context: &Context, analysis: &mut FileAnalysis) {
    let Some(function) = node.child_by_field_name("function") else { return };
    let callee = match function.kind() {
        "identifier" => Callee::Name(context.text(function)),
        "selector_expression" => {
            let operand = function.child_by_field_name("operand").filter(|operand| operand.kind() == "identifier");
            let Some(operand) = operand else { return };
            Callee::Member { object: context.text(operand), property: context.field_text(function, "field").unwrap_or_default() }
        }
        _ => return,
    };
    analysis.calls.push(Call { at: node.start_byte(), callee });
}

pub fn collect(root: Node, context: &Context, analysis: &mut FileAnalysis) {
    walk(root, |node| {
            match node.kind() {
                "import_spec" => import_spec(node, context, analysis),
                "call_expression" if context.functions => call(node, context, analysis),
                "function_declaration" | "method_declaration" if context.functions => {
                    if let Some(name) = context.field_text(node, "name") {
                        let (owner, variable) = receiver(node, context);
                        let kind = if owner.is_some() { "method" } else { "function" };
                        let exported = name.chars().next().map_or(false, char::is_uppercase);
                        let mut definition = context.definition(node, name, kind, owner, exported);
                        definition.receiver = variable;
                        analysis.definitions.push(definition);
                    }
                }
                _ => {}
            }
            true
    });
}
