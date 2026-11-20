pub mod words;
mod manifest;
#[path = "license/match.rs"]
mod matcher;
#[cfg(test)]
mod tests;

use crate::git::{Git, TreeEntry};
use manifest::{declared_license, expression_ids, identifier, notice, MANIFESTS};
use matcher::{find, match_text};
use crate::output::{LicenseMatch, LicenseOutput};
use crate::scan::ScanError;

const MAX_LICENSE_BYTES: u64 = 262_144;
const TEXT_EXTENSIONS: &[&str] = &["", "md", "markdown", "txt", "rst", "adoc", "asciidoc", "org", "html", "htm"];
const NAMES: &[&str] = &["license", "licence", "licenses", "licences", "copying", "unlicense", "copyright"];

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn is_license_file(entry: &TreeEntry) -> bool {
    if entry.path.contains('/') || entry.kind != "blob" || entry.size.unwrap_or(0) > MAX_LICENSE_BYTES {
        return false;
    }
    let name = file_name(&entry.path).to_lowercase();
    let stem = match name.rsplit_once('.') {
        Some((stem, extension)) if TEXT_EXTENSIONS.contains(&extension) => stem.to_string(),
        Some(_) if name.starts_with("license") || name.starts_with("licence") => name.clone(),
        Some(_) => return false,
        None => name.clone(),
    };
    NAMES.iter().any(|word| {
            let prefixed = ['-', '.', '_'].iter().any(|separator| stem.starts_with(&format!("{}{}", word, separator)));
            let suffixed = ['-', '_'].iter().any(|separator| stem.ends_with(&format!("{}{}", separator, word)));
            stem == *word || prefixed || suffixed
    })
}

fn links(id: &str) -> (Option<String>, Option<String>) {
    match find(id) {
        Some(template) => {
            let url = (!template.url.is_empty()).then(|| template.url.to_string());
            (url, Some(format!("https://spdx.org/licenses/{}.html", template.id)))
        }
        None => (None, None),
    }
}

fn matched(id: String, name: String, osi_approved: bool, source: &'static str, path: Option<String>, confidence: f64) -> LicenseMatch {
    let (url, spdx_url) = links(&id);
    LicenseMatch { id, name, osi_approved, source, path, confidence, url, spdx_url }
}

fn described(expression: &str, source: &'static str, path: Option<String>, confidence: f64) -> LicenseMatch {
    match find(expression) {
        Some(template) => matched(template.id.to_string(), template.name.to_string(), template.osi, source, path, confidence),
        None => matched(expression.to_string(), expression.to_string(), false, source, path, confidence),
    }
}

fn read_texts(git: &Git, entries: &[&TreeEntry]) -> Result<Vec<String>, ScanError> {
    let objects: Vec<String> = entries.iter().map(|entry| entry.object.clone()).collect();
    let mut texts = vec![String::new(); objects.len()];
    git.read_blobs(&objects, |index, content| texts[index] = String::from_utf8_lossy(&content).to_string())?;
    Ok(texts)
}

fn push_unique(licenses: &mut Vec<LicenseMatch>, item: LicenseMatch) {
    if !licenses.iter().any(|existing| existing.id == item.id) {
        licenses.push(item);
    }
}

fn file_matches(text: &str, path: &str, declared: Option<&String>, declared_ids: &[String]) -> Vec<LicenseMatch> {
    let owned = Some(path.to_string());
    if let Some(expression) = identifier(text) {
        return expression_ids(&expression).iter().map(|id| described(id, "identifier", owned.clone(), 100.0)).collect();
    }
    let found = match_text(text, declared_ids);
    if !found.is_empty() {
        let to_match = |(id, name, osi, confidence)| matched(id, name, osi, "file", owned.clone(), confidence);
        return found.into_iter().map(to_match).collect();
    }
    if let Some(id) = notice(text) {
        return vec![described(id, "notice", owned, 95.0)];
    }
    match declared {
        Some(expression) => expression_ids(expression).iter().map(|id| described(id, "manifest", owned.clone(), 0.0)).collect(),
        None => vec![matched("NOASSERTION".to_string(), "Other".to_string(), false, "file", owned, 0.0)],
    }
}

fn same_license(left: &str, right: &str) -> bool {
    let base = |id: &str| id.trim_end_matches("-only").trim_end_matches("-or-later").to_ascii_lowercase();
    let suffixed = |id: &str| id.ends_with("-only") || id.ends_with("-or-later");
    left.eq_ignore_ascii_case(right) || (base(left) == base(right) && !(suffixed(left) && suffixed(right)))
}

fn agreement(licenses: &[LicenseMatch], declared_ids: &[String], has_files: bool) -> &'static str {
    let recognised = |item: &&LicenseMatch| item.source != "manifest" && item.id != "NOASSERTION";
    let detected: Vec<&str> = licenses.iter().filter(recognised).map(|item| item.id.as_str()).collect();
    match (declared_ids.is_empty(), has_files, detected.is_empty()) {
        (true, false, _) => "none",
        (true, true, _) => "undeclared",
        (false, false, _) => "no-file",
        (false, true, true) => "unverified",
        (false, true, false) => {
            let covered = |ids: &[&str], others: &[&str]| ids.iter().all(|id| others.iter().any(|other| same_license(id, other)));
            let declared: Vec<&str> = declared_ids.iter().map(String::as_str).collect();
            if covered(&declared, &detected) && covered(&detected, &declared) {
                "match"
            } else if declared.iter().any(|id| detected.iter().any(|other| same_license(id, other))) {
                "partial"
            } else {
                "mismatch"
            }
        }
    }
}

pub fn detect(git: &Git, entries: &[TreeEntry]) -> Result<LicenseOutput, ScanError> {
    let is_manifest = |entry: &&TreeEntry| entry.kind == "blob" && MANIFESTS.contains(&entry.path.as_str());
    let manifests: Vec<&TreeEntry> = entries.iter().filter(is_manifest).collect();
    let mut declared: Option<(String, String)> = None;
    for (entry, text) in manifests.iter().zip(read_texts(git, &manifests)?) {
        if declared.is_none() {
            declared = declared_license(&entry.path, &text).map(|expression| (expression, entry.path.clone()));
        }
    }
    let (declared, declared_path) = declared.map_or((None, None), |(expression, path)| (Some(expression), Some(path)));
    let declared_ids = declared.as_deref().map(expression_ids).unwrap_or_default();
    let mut files: Vec<&TreeEntry> = entries.iter().filter(|entry| is_license_file(entry)).collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut licenses = Vec::new();
    for (entry, text) in files.iter().zip(read_texts(git, &files)?) {
        for item in file_matches(&text, &entry.path, declared.as_ref(), &declared_ids) {
            push_unique(&mut licenses, item);
        }
    }
    if licenses.is_empty() {
        for id in &declared_ids {
            push_unique(&mut licenses, described(id, "manifest", None, 0.0));
        }
    }
    let agreement = agreement(&licenses, &declared_ids, !files.is_empty());
    let files = files.iter().map(|entry| entry.path.clone()).collect();
    Ok(LicenseOutput { licenses, declared, declared_path, agreement, files })
}
