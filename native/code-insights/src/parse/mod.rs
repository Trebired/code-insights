mod go;
mod javascript;
mod python;
mod rust;

use std::cell::RefCell;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Grammar {
    Go,
    JavaScript,
    Python,
    Rust,
    Tsx,
    TypeScript,
}

impl Grammar {
    pub fn for_path(path: &str) -> Option<Grammar> {
        let name = path.rsplit('/').next().unwrap_or(path);
        if name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts") {
            return None;
        }
        let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase())?;
        match extension.as_str() {
            "ts" | "mts" | "cts" => Some(Grammar::TypeScript),
            "tsx" => Some(Grammar::Tsx),
            "js" | "jsx" | "mjs" | "cjs" => Some(Grammar::JavaScript),
            "py" | "pyi" => Some(Grammar::Python),
            "rs" => Some(Grammar::Rust),
            "go" => Some(Grammar::Go),
            _ => None,
        }
    }

    pub fn family(self) -> Family {
        match self {
            Grammar::TypeScript | Grammar::Tsx | Grammar::JavaScript => Family::Script,
            Grammar::Python => Family::Python,
            Grammar::Rust => Family::Rust,
            Grammar::Go => Family::Go,
        }
    }

    fn language(self) -> tree_sitter::Language {
        match self {
            Grammar::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Grammar::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Grammar::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Grammar::Python => tree_sitter_python::LANGUAGE.into(),
            Grammar::Rust => tree_sitter_rust::LANGUAGE.into(),
            Grammar::Go => tree_sitter_go::LANGUAGE.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Family {
    Go,
    Python,
    Rust,
    Script,
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub imported: String,
    pub local: String,
}

#[derive(Clone, Debug)]
pub struct ImportRef {
    pub bindings: Vec<Binding>,
    pub kind: &'static str,
    pub spec: String,
}

#[derive(Clone, Debug)]
pub struct Definition {
    pub end_byte: usize,
    pub end_line: usize,
    pub exported: bool,
    pub kind: &'static str,
    pub line: usize,
    pub name: String,
    pub owner: Option<String>,
    pub receiver: Option<String>,
    pub start_byte: usize,
}

#[derive(Clone, Debug)]
pub enum Callee {
    Member { object: String, property: String },
    Name(String),
    Path(Vec<String>),
    SelfMember(String),
}

#[derive(Clone, Debug)]
pub struct Call {
    pub at: usize,
    pub callee: Callee,
}

#[derive(Clone, Debug, Default)]
pub struct FileAnalysis {
    pub calls: Vec<Call>,
    pub default_export: Option<String>,
    pub definitions: Vec<Definition>,
    pub exports: HashMap<String, String>,
    pub imports: Vec<ImportRef>,
}

thread_local! {
    static PARSERS: RefCell<HashMap<Grammar, Parser>> = RefCell::new(HashMap::new());
}

pub fn analyse(grammar: Grammar, source: &str, functions: bool) -> FileAnalysis {
    PARSERS.with(|cell| {
            let mut parsers = cell.borrow_mut();
            let parser = parsers.entry(grammar).or_insert_with(|| {
                    let mut parser = Parser::new();
                    let _ = parser.set_language(&grammar.language());
                    parser
            });
            let Some(tree) = parser.parse(source, None) else { return FileAnalysis::default() };
            let mut analysis = FileAnalysis::default();
            let context = Context { functions, source: source.as_bytes() };
            match grammar.family() {
                Family::Script => javascript::collect(tree.root_node(), &context, &mut analysis),
                Family::Python => python::collect(tree.root_node(), &context, &mut analysis),
                Family::Rust => rust::collect(tree.root_node(), &context, &mut analysis),
                Family::Go => go::collect(tree.root_node(), &context, &mut analysis),
            }
            analysis
    })
}

pub struct Context<'a> {
    pub functions: bool,
    pub source: &'a [u8],
}

impl<'a> Context<'a> {
    pub fn text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    pub fn field_text(&self, node: Node, field: &str) -> Option<String> {
        node.child_by_field_name(field).map(|child| self.text(child))
    }

    pub fn string_value(&self, node: Node) -> String {
        let raw = self.text(node);
        raw.trim_matches(|character| character == '"' || character == '\'' || character == '`').to_string()
    }

    pub fn definition(&self, node: Node, name: String, kind: &'static str, owner: Option<String>, exported: bool) -> Definition {
        Definition {
            end_byte: node.end_byte(),
            end_line: node.end_position().row + 1,
            exported,
            kind,
            line: node.start_position().row + 1,
            name,
            owner,
            receiver: None,
            start_byte: node.start_byte(),
        }
    }
}

pub fn walk<F: FnMut(Node) -> bool>(root: Node, mut visit: F) {
    let mut cursor = root.walk();
    let mut descend = true;
    loop {
        if descend {
            descend = visit(cursor.node());
            if descend && cursor.goto_first_child() {
                continue;
            }
        }
        if cursor.goto_next_sibling() {
            descend = true;
            continue;
        }
        if !cursor.goto_parent() {
            return;
        }
        descend = false;
    }
}

pub fn ancestor_of_kind<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.contains(&parent.kind()) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}
