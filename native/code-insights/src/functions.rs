use std::collections::HashMap;

use crate::imports::{Parsed, ResolvedImports};
use crate::output::{CallEdge, FunctionGraph, FunctionNode};
use crate::parse::{Callee, Family};
use crate::resolve::{dir_of, Resolver};

struct Index {
    by_file: Vec<Vec<usize>>,
    defaults: HashMap<usize, usize>,
    file_of: HashMap<String, usize>,
    methods: HashMap<(usize, String, String), usize>,
    top: HashMap<(usize, String), usize>,
}

struct Binding {
    files: Vec<usize>,
    imported: String,
}

fn build_index(parsed: &[Parsed], graph: &mut FunctionGraph, max_nodes: usize) -> Index {
    let mut index = Index {
        by_file: vec![Vec::new(); parsed.len()],
        defaults: HashMap::new(),
        file_of: HashMap::new(),
        methods: HashMap::new(),
        top: HashMap::new(),
    };
    for (file, item) in parsed.iter().enumerate() {
        index.file_of.insert(item.path.clone(), file);
        for definition in &item.analysis.definitions {
            if graph.nodes.len() >= max_nodes {
                graph.truncated = true;
                break;
            }
            let id = graph.nodes.len();
            graph.nodes.push(FunctionNode {
                    file, name: definition.name.clone(), kind: definition.kind, owner: definition.owner.clone(),
                    line: definition.line, end_line: definition.end_line, exported: definition.exported,
            });
            index.by_file[file].push(id);
            match &definition.owner {
                Some(owner) => { index.methods.entry((file, owner.clone(), definition.name.clone())).or_insert(id); }
                None => { index.top.entry((file, definition.name.clone())).or_insert(id); }
            }
            if item.analysis.default_export.as_deref() == Some(definition.name.as_str()) && definition.owner.is_none() {
                index.defaults.insert(file, id);
            }
        }
    }
    index
}

fn bindings_for(file: usize, parsed: &[Parsed], resolved: &ResolvedImports, index: &Index) -> HashMap<String, Binding> {
    let mut bindings = HashMap::new();
    for (position, targets) in &resolved[file] {
        let reference = &parsed[file].analysis.imports[*position];
        let files: Vec<usize> = targets.iter().filter_map(|target| index.file_of.get(target).copied()).collect();
        for binding in &reference.bindings {
            bindings.insert(binding.local.clone(), Binding { files: files.clone(), imported: binding.imported.clone() });
        }
    }
    bindings
}

fn caller_of(at: usize, definitions: &[crate::parse::Definition], ids: &[usize]) -> Option<(usize, usize)> {
    definitions.iter().enumerate().filter(|(_, definition)| definition.start_byte <= at && at < definition.end_byte)
    .min_by_key(|(_, definition)| definition.end_byte - definition.start_byte)
    .and_then(|(position, _)| ids.get(position).map(|id| (position, *id)))
}

fn in_files(files: &[usize], name: &str, index: &Index) -> Option<usize> {
    files.iter().find_map(|file| {
            if name == "default" { index.defaults.get(file).copied() } else { index.top.get(&(*file, name.to_string())).copied() }
    })
}

fn method_in(files: &[usize], owner: &str, name: &str, index: &Index) -> Option<usize> {
    files.iter().find_map(|file| index.methods.get(&(*file, owner.to_string(), name.to_string())).copied())
}

struct CallContext<'a> {
    bindings: &'a HashMap<String, Binding>,
    file: usize,
    owner: Option<&'a str>,
    package: &'a [usize],
    receiver: Option<&'a str>,
}

fn resolve_call(callee: &Callee, context: &CallContext, family: Family, index: &Index) -> Option<(usize, &'static str)> {
    let same = [context.file];
    match callee {
        Callee::SelfMember(name) => context.owner.and_then(|owner| method_in(&same, owner, name, index)).map(|id| (id, "member")),
        Callee::Name(name) => {
            if let Some(id) = in_files(&same, name, index) {
                return Some((id, "local"));
            }
            if family == Family::Go {
                if let Some(id) = in_files(context.package, name, index) {
                    return Some((id, "package"));
                }
            }
            let binding = context.bindings.get(name)?;
            in_files(&binding.files, &binding.imported, index).map(|id| (id, "import"))
        }
        Callee::Member { object, property } => {
            if family == Family::Go && context.receiver == Some(object.as_str()) {
                return context.owner.and_then(|owner| method_in(context.package, owner, property, index)).map(|id| (id, "member"));
            }
            let binding = context.bindings.get(object)?;
            if binding.imported == "*" || family == Family::Go {
                return in_files(&binding.files, property, index).map(|id| (id, "import"));
            }
            method_in(&binding.files, &binding.imported, property, index).map(|id| (id, "import"))
        }
        Callee::Path(segments) => {
            let name = segments.last()?;
            let owner = segments.get(segments.len().checked_sub(2)?)?;
            let owner = if owner == "Self" { context.owner? } else { owner.as_str() };
            if let Some(id) = method_in(&same, owner, name, index) {
                return Some((id, "member"));
            }
            let binding = context.bindings.get(owner)?;
            method_in(&binding.files, &binding.imported, name, index).or_else(|| in_files(&binding.files, name, index)).map(|id| (id, "import"))
        }
    }
}

pub fn build(
    parsed: &[Parsed],
    resolved: &ResolvedImports,
    resolver: &Resolver,
    max_nodes: usize,
    max_edges: usize,
    tick: &dyn Fn()
) -> FunctionGraph {
    let mut graph = FunctionGraph { files: parsed.iter().map(|item| item.path.clone()).collect(), ..FunctionGraph::default() };
    let index = build_index(parsed, &mut graph, max_nodes);
    let mut edges: HashMap<(usize, usize), (usize, &'static str)> = HashMap::new();
    for (file, item) in parsed.iter().enumerate() {
        let bindings = bindings_for(file, parsed, resolved, &index);
        let package: Vec<usize> = if item.grammar.family() == Family::Go {
            resolver.go_package_files(dir_of(&item.path)).iter().filter_map(|path| index.file_of.get(path).copied()).collect()
        } else {
            Vec::new()
        };
        let definitions = &item.analysis.definitions;
        for call in &item.analysis.calls {
            let Some((position, caller)) = caller_of(call.at, definitions, &index.by_file[file]) else { continue };
            let context = CallContext {
                bindings: &bindings, file, owner: definitions[position].owner.as_deref(),
                package: &package, receiver: definitions[position].receiver.as_deref(),
            };
            match resolve_call(&call.callee, &context, item.grammar.family(), &index) {
                Some((target, confidence)) if target != caller => {
                    edges.entry((caller, target)).or_insert((0, confidence)).0 += 1;
                }
                Some(_) => {}
                None => graph.unresolved += 1,
            }
        }
        tick();
    }
    let mut list: Vec<CallEdge> = edges.into_iter()
    .map(|((from, to), (count, confidence))| CallEdge { from, to, confidence, count: count as u32 })
    .collect();
    list.sort_by(|left, right| right.count.cmp(&left.count).then(left.from.cmp(&right.from)).then(left.to.cmp(&right.to)));
    if list.len() > max_edges {
        list.truncate(max_edges);
        graph.truncated = true;
    }
    graph.edges = list;
    graph
}
