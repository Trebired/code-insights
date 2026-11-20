use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use crate::filter::{Exclusion, Filter};
use crate::git::{Git, TreeEntry};
use crate::imports::Parsed;
use crate::output::{FileRecord, LanguageTotal, Lines, ScanResult, Totals, RESULT_VERSION};
use crate::parse::Grammar;
use crate::progress::Reporter;
use crate::resolve::Resolver;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Listing,
    License,
    Languages,
    Tree,
    Imports,
    Functions,
    Hotspots,
}

impl Phase {
    pub const ALL: [Phase; 7] = [Phase::Listing, Phase::License, Phase::Languages, Phase::Tree, Phase::Imports, Phase::Functions, Phase::Hotspots];

    pub fn name(self) -> &'static str {
        match self {
            Phase::Listing => "listing",
            Phase::License => "license",
            Phase::Languages => "languages",
            Phase::Tree => "tree",
            Phase::Imports => "imports",
            Phase::Functions => "functions",
            Phase::Hotspots => "hotspots",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Phase::Listing => 5.0,
            Phase::License => 1.0,
            Phase::Languages => 40.0,
            Phase::Tree => 5.0,
            Phase::Imports => 30.0,
            Phase::Functions => 10.0,
            Phase::Hotspots => 10.0,
        }
    }
}

pub fn parse_phases(input: Option<&str>) -> Vec<Phase> {
    let Some(input) = input.filter(|value| !value.trim().is_empty()) else { return Phase::ALL.to_vec() };
    let wanted: Vec<&str> = input.split(',').map(str::trim).collect();
    let mut phases: Vec<Phase> = Phase::ALL.iter().copied().filter(|phase| *phase == Phase::Listing || wanted.contains(&phase.name())).collect();
    if phases.contains(&Phase::Functions) {
        phases.push(Phase::Imports);
    }
    if phases.iter().any(|phase| matches!(phase, Phase::Imports | Phase::Hotspots)) {
        phases.push(Phase::Languages);
    }
    Phase::ALL.iter().copied().filter(|phase| phases.contains(phase)).collect()
}

pub struct ScanOptions {
    pub git_dir: PathBuf,
    pub history: usize,
    pub max_function_edges: usize,
    pub max_function_nodes: usize,
    pub max_import_edges: usize,
    pub max_parse_bytes: usize,
    pub max_read_bytes: usize,
    pub phases: Vec<Phase>,
    pub reference: String,
}

#[derive(Debug)]
pub struct ScanError {
    pub code: String,
    pub message: String,
}

impl ScanError {
    pub fn new(code: &str, message: &str) -> Self {
        ScanError { code: code.to_string(), message: message.to_string() }
    }
}

struct Candidate {
    entry: TreeEntry,
    content: Option<Vec<u8>>,
}

struct Detected {
    excluded: Option<Exclusion>,
    grammar: Option<Grammar>,
    language: Option<&'static str>,
    record: FileRecord,
    text: Option<String>,
}

fn is_config(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.ends_with(".json") || name == "go.mod" || name == "Cargo.toml"
}

fn iso_now() -> String {
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|value| value.as_secs() as i64).unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, rest / 3600, rest % 3600 / 60, rest % 60)
}

fn timed<T>(timings: &mut BTreeMap<String, u64>, reporter: &Reporter, phase: Phase, total: u64, work: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    reporter.start(phase, total);
    let value = work();
    reporter.finish(phase);
    timings.insert(phase.name().to_string(), started.elapsed().as_millis() as u64);
    value
}

fn detect_one(candidate: Candidate, filter: &Filter, wants_parse: bool, max_parse: usize) -> Detected {
    let path = candidate.entry.path.clone();
    let bytes = candidate.entry.size.unwrap_or(0);
    let record = FileRecord { path: path.clone(), language: None, bytes, lines: Lines::default() };
    let mut detected = Detected { excluded: None, grammar: None, language: None, record, text: None };
    let Some(content) = candidate.content else {
        detected.language = crate::lang::detect(&path, "");
        return detected;
    };
    if crate::lines::is_binary(&content) {
        detected.excluded = Some(Exclusion::Vendored);
        detected.record.language = Some("\u{0}binary".to_string());
        return detected;
    }
    let text = String::from_utf8(content).unwrap_or_default();
    if filter.generated_by_content(&path, &text) {
        detected.excluded = Some(Exclusion::Generated);
        return detected;
    }
    let overridden = filter.language_override(&path)
    .and_then(|name| hyperpolyglot::Language::try_from(name.as_str()).ok().map(|language| language.name));
    detected.language = overridden.or_else(|| crate::lang::detect(&path, &text));
    detected.record.lines = crate::lines::count(&text);
    detected.grammar = Grammar::for_path(&path);
    if wants_parse && detected.grammar.is_some() && text.len() <= max_parse {
        detected.text = Some(text);
    }
    detected
}

fn tally(detected: &[Detected], result: &mut ScanResult) {
    let mut by_language: BTreeMap<&'static str, Totals> = BTreeMap::new();
    for item in detected {
        match item.excluded {
            Some(Exclusion::Generated) => { result.skipped.generated += 1; continue; }
            Some(_) if item.record.language.as_deref() == Some("\u{0}binary") => { result.skipped.binary += 1; continue; }
            Some(_) => continue,
            None => {}
        }
        let totals = match item.language {
            Some(language) => by_language.entry(language).or_default(),
            None => &mut result.unknown,
        };
        totals.bytes += item.record.bytes;
        totals.count += 1;
        totals.lines.add(&item.record.lines);
        result.totals.bytes += item.record.bytes;
        result.totals.count += 1;
        result.totals.lines.add(&item.record.lines);
        result.files.push(FileRecord { language: item.language.map(|name| crate::lang::info(name).name), ..item.record.clone() });
    }
    let known_bytes: u64 = by_language.values().map(|totals| totals.bytes).sum();
    let mut languages: Vec<LanguageTotal> = by_language.into_iter().map(|(name, totals)| {
            let info = crate::lang::info(name);
            LanguageTotal {
                id: crate::lang::language_id(name), name: info.name, kind: info.kind.to_string(), color: info.color,
                bytes: totals.bytes, count: totals.count, lines: totals.lines,
                percent: if known_bytes == 0 { 0.0 } else { ((totals.bytes as f64 / known_bytes as f64) * 1000.0).round() / 10.0 },
            }
    }).collect();
    languages.sort_by(|left, right| right.bytes.cmp(&left.bytes).then(left.name.cmp(&right.name)));
    result.languages = languages;
}

pub fn scan_repository(options: &ScanOptions, reporter: &Reporter) -> Result<ScanResult, ScanError> {
    let git = Git::open(&options.git_dir)?;
    let mut result = ScanResult { version: RESULT_VERSION, reference: options.reference.clone(), scanned_at: iso_now(), ..ScanResult::default() };
    let wants = |phase: Phase| options.phases.contains(&phase);
    let mut timings = BTreeMap::new();
    let listed = timed(&mut timings, reporter, Phase::Listing, 1, || -> Result<Option<(String, Vec<TreeEntry>)>, ScanError> {
            let Some(commit) = git.resolve_commit(&options.reference)? else { return Ok(None) };
            let entries = git.list_tree(&commit)?;
            Ok(Some((commit, entries)))
    })?;
    let Some((commit, entries)) = listed else {
        result.empty = true;
        result.timings_ms = timings;
        return Ok(result);
    };
    result.commit = commit.clone();
    let blobs: Vec<&TreeEntry> = entries.iter().filter(|entry| entry.kind == "blob" && entry.mode != "120000").collect();
    let mut gitattributes = None;
    if let Some(entry) = blobs.iter().find(|entry| entry.path == ".gitattributes") {
        git.read_blobs(&[entry.object.clone()], |_, content| gitattributes = Some(String::from_utf8_lossy(&content).to_string()))?;
    }
    if wants(Phase::License) {
        result.license = Some(timed(&mut timings, reporter, Phase::License, 1, || crate::license::detect(&git, &entries))?);
    }
    let filter = Filter::new(gitattributes.as_deref());
    let mut detected: Vec<Detected> = Vec::new();
    let mut configs: HashMap<String, String> = HashMap::new();
    let wants_parse = wants(Phase::Imports) || wants(Phase::Functions);
    if wants(Phase::Languages) {
        detected = timed(&mut timings, reporter, Phase::Languages, blobs.len() as u64, || -> Result<Vec<Detected>, ScanError> {
                read_and_detect(&git, &blobs, &filter, options, wants_parse, reporter, &mut result, &mut configs)
        })?;
        tally(&detected, &mut result);
    }
    if wants(Phase::Tree) {
        let languages: HashMap<String, String> = result.files.iter().filter_map(|file| Some((file.path.clone(), file.language.clone()?))).collect();
        let tree = timed(&mut timings, reporter, Phase::Tree, 1, || crate::tree::build(&entries, &languages));
        result.tree = Some(tree);
    }
    if wants_parse {
        analyse_graphs(&mut result, &mut timings, reporter, options, detected, configs, blobs.iter().map(|entry| entry.path.clone()).collect());
    }
    if wants(Phase::Hotspots) {
        let churn = timed(&mut timings, reporter, Phase::Hotspots, 1, || git.churn(&commit, options.history))?;
        let programming = |name: &str| crate::lang::info(name).kind == "programming";
        result.hotspots = Some(crate::hotspots::build(&result.files, &churn, &programming, 20));
    }
    result.timings_ms = timings;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn read_and_detect(
    git: &Git, blobs: &[&TreeEntry], filter: &Filter, options: &ScanOptions, wants_parse: bool,
    reporter: &Reporter, result: &mut ScanResult, configs: &mut HashMap<String, String>,
) -> Result<Vec<Detected>, ScanError> {
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut to_read: Vec<usize> = Vec::new();
    for entry in blobs {
        let exclusion = filter.exclusion_by_path(&entry.path);
        let config = is_config(&entry.path) && entry.size.unwrap_or(0) <= 2_097_152;
        match exclusion {
            Some(Exclusion::Vendored) => result.skipped.vendored += 1,
            Some(Exclusion::Generated) => result.skipped.generated += 1,
            Some(Exclusion::Documentation) => result.skipped.documentation += 1,
            None => {}
        }
        if exclusion.is_some() && !config {
            reporter.tick(1);
            continue;
        }
        if entry.size.unwrap_or(0) as usize <= options.max_read_bytes {
            to_read.push(candidates.len());
        } else {
            result.skipped.unparsed += 1;
        }
        candidates.push(Candidate { entry: (*entry).clone(), content: None });
        if exclusion.is_some() {
            candidates.last_mut().unwrap().entry.mode = "excluded".to_string();
        }
    }
    let objects: Vec<String> = to_read.iter().map(|index| candidates[*index].entry.object.clone()).collect();
    git.read_blobs(&objects, |position, content| {
            let candidate = &mut candidates[to_read[position]];
            if is_config(&candidate.entry.path) {
                configs.insert(candidate.entry.path.clone(), String::from_utf8_lossy(&content).to_string());
            }
            candidate.content = Some(content);
    })?;
    let max_parse = options.max_parse_bytes;
    let detected: Vec<Detected> = candidates.into_par_iter().filter(|candidate| candidate.entry.mode != "excluded").map(|candidate| {
            let item = detect_one(candidate, filter, wants_parse, max_parse);
            reporter.tick(1);
            item
    }).collect();
    Ok(detected)
}

#[allow(clippy::too_many_arguments)]
fn analyse_graphs(
    result: &mut ScanResult,
    timings: &mut BTreeMap<String,
    u64>,
    reporter: &Reporter,
    options: &ScanOptions,
    detected: Vec<Detected>,
    configs: HashMap<String,
    String>,
    all_paths: Vec<String>
) {
    let functions = options.phases.contains(&Phase::Functions);
    let sources: Vec<(
        String,
        Grammar,
        String
    )> = detected.into_iter().filter_map(|item| Some((item.record.path, item.grammar?, item.text?))).collect();
    let resolver = Resolver::new(&all_paths, configs);
    let (graph, parsed, resolved) = timed(timings, reporter, Phase::Imports, sources.len() as u64, || {
            let parsed: Vec<Parsed> = sources.into_par_iter().map(|(path, grammar, text)| {
                    let analysis = crate::parse::analyse(grammar, &text, functions);
                    reporter.tick(1);
                    Parsed { analysis, grammar, path }
            }).collect();
            let (graph, resolved) = crate::imports::build(&parsed, &resolver, options.max_import_edges);
            (graph, parsed, resolved)
    });
    result.imports = Some(graph);
    if functions {
        let tick = || reporter.tick(1);
        let graph = timed(timings, reporter, Phase::Functions, parsed.len() as u64, || {
                crate::functions::build(&parsed, &resolved, &resolver, options.max_function_nodes, options.max_function_edges, &tick)
        });
        result.functions = Some(graph);
    }
}
