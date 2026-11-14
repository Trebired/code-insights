use std::convert::TryFrom;
use std::io::Cursor;

use hyperpolyglot::detectors;
use hyperpolyglot::{Language, LanguageType};

const MAX_CLASSIFY_BYTES: usize = 51_200;

fn truncate(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn narrow(candidates: Vec<&'static str>, next: Vec<&'static str>) -> Vec<&'static str> {
    if candidates.is_empty() {
        return next;
    }
    let filtered: Vec<&'static str> = candidates.iter().copied().filter(|item| next.contains(item)).collect();
    if filtered.is_empty() { candidates } else { filtered }
}

pub fn detect(path: &str, content: &str) -> Option<&'static str> {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if let Some(language) = detectors::get_language_from_filename(filename) {
        return Some(language);
    }
    let extension = detectors::get_extension(filename);
    let mut candidates = extension.map(detectors::get_languages_from_extension).unwrap_or_default();
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let shebang = detectors::get_languages_from_shebang(Cursor::new(content.as_bytes())).unwrap_or_default();
    candidates = narrow(candidates, shebang);
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let sample = truncate(content, MAX_CLASSIFY_BYTES);
    if candidates.len() > 1 {
        if let Some(extension) = extension {
            let heuristic = detectors::get_languages_from_heuristics(extension, &candidates, sample);
            candidates = narrow(candidates, heuristic);
        }
    }
    match candidates.len() {
        0 => None,
        1 => Some(candidates[0]),
        _ => Some(detectors::classify(sample, &candidates)),
    }
}

pub struct LanguageInfo {
    pub name: String,
    pub color: String,
    pub kind: &'static str,
}

pub fn info(name: &str) -> LanguageInfo {
    match Language::try_from(name) {
        Ok(language) => LanguageInfo {
            name: language.name.to_string(),
            color: language.color.unwrap_or("").to_string(),
            kind: match language.language_type {
                LanguageType::Data => "data",
                LanguageType::Markup => "markup",
                LanguageType::Programming => "programming",
                LanguageType::Prose => "prose",
            },
        },
        Err(_) => LanguageInfo { name: name.to_string(), color: String::new(), kind: "programming" },
    }
}

pub fn describe(name: &'static str) -> serde_json::Value {
    let info = info(name);
    serde_json::json!({ "name": info.name, "color": info.color, "type": info.kind, "id": language_id(name) })
}

pub fn language_id(name: &str) -> String {
    let lowered = name.to_lowercase().replace("++", "pp").replace('#', "sharp");
    let mut id = String::new();
    let mut dash = false;
    for character in lowered.chars() {
        if character.is_alphanumeric() || character == '_' {
            id.push(character);
            dash = false;
        } else if !dash {
            id.push('-');
            dash = true;
        }
    }
    let trimmed = id.trim_matches('-').to_string();
    if trimmed.is_empty() { "other".to_string() } else { trimmed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_languages() {
        assert_eq!(detect("src/app.ts", "const a = 1;"), Some("TypeScript"));
        assert_eq!(detect("main.rs", "fn main() {}"), Some("Rust"));
        assert_eq!(detect("Dockerfile", "FROM alpine"), Some("Dockerfile"));
        assert_eq!(detect("tool", "#!/usr/bin/env python3\nprint(1)"), Some("Python"));
    }

    #[test]
    fn builds_language_ids_like_the_platform() {
        assert_eq!(language_id("C++"), "cpp");
        assert_eq!(language_id("C#"), "csharp");
        assert_eq!(language_id("Vim Script"), "vim-script");
    }
}
