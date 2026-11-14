use std::collections::{HashMap, HashSet};

use crate::output::{ImportEdge, ImportGraph, UnresolvedImport};
use crate::parse::{FileAnalysis, Grammar};
use crate::resolve::{Resolution, Resolver};

pub struct Parsed {
    pub analysis: FileAnalysis,
    pub grammar: Grammar,
    pub path: String,
}

pub type ResolvedImports = Vec<Vec<(usize, Vec<String>)>>;

pub fn build(parsed: &[Parsed], resolver: &Resolver, max_edges: usize) -> (ImportGraph, ResolvedImports) {
    let mut graph = ImportGraph { nodes: parsed.iter().map(|file| file.path.clone()).collect(), ..ImportGraph::default() };
    let index: HashMap<&str, usize> = graph.nodes.iter().enumerate().map(|(position, path)| (path.as_str(), position)).collect();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut resolved: ResolvedImports = Vec::with_capacity(parsed.len());
    for (from, file) in parsed.iter().enumerate() {
        let mut per_file = Vec::new();
        for (position, reference) in file.analysis.imports.iter().enumerate() {
            match resolver.resolve(file.grammar.family(), &file.path, reference) {
                Resolution::External => graph.external += 1,
                Resolution::Unresolved => {
                    graph.unresolved += 1;
                    if graph.unresolved_samples.len() < 50 {
                        graph.unresolved_samples.push(UnresolvedImport { from, spec: reference.spec.clone() });
                    }
                }
                Resolution::Files(targets) => {
                    for target in &targets {
                        let Some(&to) = index.get(target.as_str()) else { continue };
                        if to == from || !seen.insert((from, to)) {
                            continue;
                        }
                        if graph.edges.len() >= max_edges {
                            graph.truncated = true;
                            continue;
                        }
                        graph.edges.push(ImportEdge { from, to, kind: reference.kind });
                    }
                    per_file.push((position, targets));
                }
            }
        }
        resolved.push(per_file);
    }
    (graph, resolved)
}
