use super::words::phrase;

pub const MANIFESTS: &[&str] = &["package.json", "Cargo.toml", "pyproject.toml", "composer.json"];
const MAX_NOTICE_WORDS: usize = 400;

const NOTICES: &[(&str, &[&[&str]])] = &[
    ("AGPL-3.0-or-later", &[&[" gnu affero general public license ", " version 3 ", " any later version "]]),
    ("AGPL-3.0-only", &[&[" gnu affero general public license ", " version 3 "]]),
    ("LGPL-3.0-or-later", &[&[" gnu lesser general public license ", " version 3 ", " any later version "]]),
    ("LGPL-3.0-only", &[&[" gnu lesser general public license ", " version 3 "]]),
    ("LGPL-2.1-or-later", &[&[" gnu lesser general public license ", " version 2 1 ", " any later version "]]),
    ("LGPL-2.1-only", &[&[" gnu lesser general public license ", " version 2 1 "]]),
    ("GPL-3.0-or-later", &[&[" gnu general public license ", " version 3 ", " any later version "]]),
    ("GPL-3.0-only", &[&[" gnu general public license ", " version 3 "]]),
    ("GPL-2.0-or-later", &[&[" gnu general public license ", " version 2 ", " any later version "]]),
    ("GPL-2.0-only", &[&[" gnu general public license ", " version 2 "]]),
    ("Apache-2.0", &[&[" apache license ", " 2 0 "]]),
    ("MPL-2.0", &[&[" mozilla public license ", " 2 0 "]]),
    ("EPL-2.0", &[&[" eclipse public license ", " 2 0 "]]),
    ("EPL-1.0", &[&[" eclipse public license ", " 1 0 "]]),
    ("EUPL-1.2", &[&[" eupl ", " 1 2 "], &[" european union public licen", " 1 2 "]]),
    ("CC-BY-SA-4.0", &[&[" creative commons attributionsharealike 4 0 "], &[" cc bysa 4 0 "]]),
    ("CC-BY-4.0", &[&[" creative commons attribution 4 0 "], &[" cc by 4 0 "]]),
    ("CC0-1.0", &[&[" cc0 1 0 "], &[" cc0 "], &[" creative commons zero "]]),
    ("BSD-3-Clause", &[&[" bsd3clause "], &[" 3clause bsd "], &[" new bsd license "]]),
    ("BSD-2-Clause", &[&[" bsd2clause "], &[" 2clause bsd "], &[" simplified bsd license "]]),
    ("BSL-1.0", &[&[" boost software license "]]),
    ("Unlicense", &[&[" unlicense "]]),
    ("ISC", &[&[" isc license "]]),
    ("Zlib", &[&[" zlib license "]]),
    ("MIT", &[&[" mit license "], &[" licensed under the mit "], &[" released under the mit "]]),
];

pub fn notice(text: &str) -> Option<&'static str> {
    let phrase = phrase(text);
    if phrase.split_whitespace().count() > MAX_NOTICE_WORDS {
        return None;
    }
    let matches = |parts: &&[&str]| parts.iter().all(|part| phrase.contains(part));
    NOTICES.iter().find(|(_, alternatives)| alternatives.iter().any(matches)).map(|(id, _)| *id)
}

pub fn identifier(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
            let (_, rest) = line.split_once("SPDX-License-Identifier:")?;
            let value = rest.trim().trim_end_matches("*/").trim_end_matches("-->").trim();
            (!value.is_empty()).then(|| value.to_string())
    })
}

pub fn expression_ids(expression: &str) -> Vec<String> {
    expression.split(|c: char| c == '(' || c == ')' || c.is_whitespace()).filter(|part| !part.is_empty())
    .filter(|part| !matches!(part.to_ascii_uppercase().as_str(), "OR" | "AND" | "WITH")).map(str::to_string).collect()
}

fn toml_license(text: &str, sections: &[&str]) -> Option<String> {
    let mut active = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            active = sections.contains(&line);
            continue;
        }
        let Some(rest) = line.strip_prefix("license") else { continue };
        let rest = rest.trim_start();
        if !active || !rest.starts_with('=') {
            continue;
        }
        let value = rest[1..].trim();
        let inline_text = |inner: &str| inner.split_once("text").map(|(_, after)| after.trim_start_matches([' ', '=']).to_string());
        let value = value.strip_prefix('{').and_then(inline_text)
        .unwrap_or_else(|| value.to_string());
        let quoted = value.trim().trim_start_matches(['"', '\'']);
        let end = quoted.find(['"', '\'']).unwrap_or(quoted.len());
        let license = quoted[..end].trim();
        if !license.is_empty() {
            return Some(license.to_string());
        }
    }
    None
}

fn json_license(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let single = |item: &serde_json::Value| item.as_str().map(str::to_string).or_else(|| item["type"].as_str().map(str::to_string));
    if let Some(license) = single(&value["license"]) {
        return Some(license);
    }
    let list = value["license"].as_array().or_else(|| value["licenses"].as_array())?;
    let ids: Vec<String> = list.iter().filter_map(single).collect();
    match ids.len() {
        0 => None,
        1 => ids.into_iter().next(),
        _ => Some(ids.join(" OR ")),
    }
}

pub fn declared_license(path: &str, text: &str) -> Option<String> {
    let license = match path {
        "package.json" | "composer.json" => json_license(text),
        "Cargo.toml" => toml_license(text, &["[package]", "[workspace.package]"]),
        "pyproject.toml" => toml_license(text, &["[project]", "[tool.poetry]"]),
        _ => None,
    }?;
    let trimmed = license.trim();
    let placeholder = trimmed.eq_ignore_ascii_case("UNLICENSED") || trimmed.to_ascii_uppercase().starts_with("SEE LICENSE");
    (!trimmed.is_empty() && !placeholder).then(|| trimmed.to_string())
}
