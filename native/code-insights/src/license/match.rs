use std::collections::HashSet;
use std::sync::OnceLock;

const INDEX: &str = include_str!(concat!(env!("OUT_DIR"), "/licenses.tsv"));
const MATCH_THRESHOLD: f64 = 90.0;
const TIE_MARGIN: f64 = 0.5;
const DECLARED_MARGIN: f64 = 3.0;
const MIN_CONTAINMENT: f64 = 0.97;
const MIN_DECLARED_CONTAINMENT: f64 = 0.9;
const MIN_COVERAGE: f64 = 0.6;
const MIN_COVER_WORDS: usize = 25;
const MIN_FRESH_WORDS: usize = 8;
const MAX_COVER: usize = 4;
const PREFERRED: &[&str] = &[
    "MIT", "Apache-2.0", "GPL-3.0-only", "GPL-2.0-only", "BSD-3-Clause", "BSD-2-Clause", "AGPL-3.0-only", "LGPL-3.0-only",
    "LGPL-2.1-only", "MPL-2.0", "ISC", "Unlicense", "CC0-1.0", "0BSD", "BSL-1.0", "EPL-2.0", "Zlib", "MIT-0", "EUPL-1.2", "WTFPL",
];

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub osi: bool,
    pub deprecated: bool,
    pub url: &'static str,
    pub words: Vec<&'static str>,
}

pub fn templates() -> &'static [Template] {
    static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
            INDEX.lines().filter_map(|line| {
                    let mut parts = line.splitn(6, '\t');
                    let id = parts.next()?;
                    let name = parts.next()?;
                    let osi = parts.next()? == "1";
                    let deprecated = parts.next()? == "1";
                    let url = parts.next()?;
                    let words = parts.next().unwrap_or("").split(' ').filter(|word| !word.is_empty()).collect();
                    Some(Template { id, name, osi, deprecated, url, words })
            }).collect()
    })
}

pub fn find(id: &str) -> Option<&'static Template> {
    templates().iter().find(|template| template.id.eq_ignore_ascii_case(id))
}

fn dice(words: &HashSet<&str>, template: &Template) -> f64 {
    if words.is_empty() || template.words.is_empty() {
        return 0.0;
    }
    let shared = template.words.iter().filter(|word| words.contains(*word)).count();
    (2.0 * shared as f64 / (words.len() + template.words.len()) as f64) * 100.0
}

fn family(ids: &[&'static Template]) -> Option<(String, String)> {
    let bases: HashSet<&str> = ids.iter().map(|template| template.id.trim_end_matches("-only").trim_end_matches("-or-later")).collect();
    if bases.len() != 1 || ids.len() < 2 {
        return None;
    }
    let base = *bases.iter().next()?;
    let only = ids.iter().find(|template| template.id.ends_with("-only")).copied().unwrap_or(ids[0]);
    Some((base.to_string(), only.name.trim_end_matches(" only").trim_end_matches(" or later").to_string()))
}

fn pick(tied: Vec<&'static Template>, declared: &[String]) -> (String, String, bool) {
    if let Some(template) = tied.iter().find(|template| declared.iter().any(|id| id.eq_ignore_ascii_case(template.id))) {
        return (template.id.to_string(), template.name.to_string(), template.osi);
    }
    if let Some((id, name)) = family(&tied) {
        return (id, name, tied.iter().any(|template| template.osi));
    }
    let rank = |template: &&Template| PREFERRED.iter().position(|id| *id == template.id).unwrap_or(usize::MAX);
    let best = tied.iter().min_by(|left, right| rank(left).cmp(&rank(right)).then(left.id.cmp(right.id))).copied().unwrap_or(tied[0]);
    (best.id.to_string(), best.name.to_string(), best.osi)
}

pub type Found = (String, String, bool, f64);

fn best_single(words: &HashSet<&str>, declared: &[String]) -> Option<Found> {
    let scored: Vec<(&'static Template, f64)> = templates().iter().filter(|template| !template.deprecated)
    .map(|template| (template, dice(words, template))).filter(|(_, score)| *score >= MATCH_THRESHOLD).collect();
    let best = scored.iter().map(|(_, score)| *score).fold(0.0_f64, f64::max);
    if best == 0.0 {
        return None;
    }
    let is_declared = |template: &Template| declared.iter().any(|id| id.eq_ignore_ascii_case(template.id));
    let declared_near = scored.iter().filter(|(template, score)| *score >= best - DECLARED_MARGIN && is_declared(template))
    .max_by(|left, right| left.1.total_cmp(&right.1));
    if let Some((template, score)) = declared_near {
        return Some((template.id.to_string(), template.name.to_string(), template.osi, (score * 10.0).round() / 10.0));
    }
    let tied: Vec<&'static Template> = scored.iter().filter(|(_, score)| *score >= best - TIE_MARGIN).map(|(template, _)| *template).collect();
    let (id, name, osi) = pick(tied, declared);
    Some((id, name, osi, (best * 10.0).round() / 10.0))
}

fn contained(words: &HashSet<&str>, template: &Template) -> usize {
    template.words.iter().filter(|word| words.contains(*word)).count()
}

fn covering(words: &HashSet<&str>, declared: &[String]) -> Vec<Found> {
    let is_declared = |template: &Template| declared.iter().any(|id| id.eq_ignore_ascii_case(template.id));
    let mut candidates: Vec<&'static Template> = templates().iter().filter(|template| !template.deprecated && template.words.len() >= MIN_COVER_WORDS)
    .filter(|template| {
            let needed = if is_declared(template) { MIN_DECLARED_CONTAINMENT } else { MIN_CONTAINMENT };
            contained(words, template) as f64 >= template.words.len() as f64 * needed
    }).collect();
    candidates.sort_by(|left, right| {
            is_declared(right).cmp(&is_declared(left)).then(right.words.len().cmp(&left.words.len())).then(left.id.cmp(right.id))
    });
    let mut covered: HashSet<&str> = HashSet::new();
    let mut found: Vec<Found> = Vec::new();
    for template in &candidates {
        let fresh = template.words.iter().filter(|word| words.contains(*word) && !covered.contains(*word)).count();
        if found.len() >= MAX_COVER || fresh < MIN_FRESH_WORDS.max(template.words.len() / 5) {
            continue;
        }
        covered.extend(template.words.iter().copied().filter(|word| words.contains(*word)));
        let twins: Vec<&'static Template> = candidates.iter().copied().filter(|other| other.words == template.words).collect();
        let (id, name, osi) = pick(twins, declared);
        let share = contained(words, template) as f64 / template.words.len() as f64 * 100.0;
        found.push((id, name, osi, (share * 10.0).round() / 10.0));
    }
    if (covered.len() as f64) < words.len() as f64 * MIN_COVERAGE {
        return Vec::new();
    }
    found
}

pub fn match_text(text: &str, declared: &[String]) -> Vec<Found> {
    let owned = super::words::words(text);
    let words: HashSet<&str> = owned.iter().map(String::as_str).collect();
    let covered = covering(&words, declared);
    match best_single(&words, declared) {
        Some(found) if covered.len() >= 2 && covered.iter().any(|item| same_text(&item.0, &found.0)) => covered,
        Some(found) => vec![found],
        None => covered,
    }
}

fn same_text(left: &str, right: &str) -> bool {
    let words = |id: &str| find(id).or_else(|| find(&format!("{}-only", id))).map(|template| &template.words);
    left == right || words(left).is_some_and(|words_left| words(right) == Some(words_left))
}
