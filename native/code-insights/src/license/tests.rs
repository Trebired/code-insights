use super::manifest::{declared_license, identifier, notice};
use super::matcher::{find, match_text, templates};

fn text_of(id: &str) -> String {
    let template = find(id).expect("known license");
    template.words.join(" ")
}

#[test]
fn every_license_matches_itself() {
    for template in templates().iter().filter(|template| !template.deprecated && !template.words.is_empty()) {
        let found = match_text(&template.words.join(" "), &[template.id.to_string()]);
        assert_eq!(found.first().map(|item| item.0.as_str()), Some(template.id), "{} matched {:?}", template.id, found);
    }
}

fn real_text(id: &str) -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../vendor/spdx-license-list/spdx-full.json");
    let list: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let text = list[id]["licenseText"].as_str().unwrap().replace("<year> <copyright holders>", "2026 Jane Doe");
    format!("# License\n\nCopyright (c) 2026 Jane Doe. All rights reserved.\n\n{}", text)
}

#[test]
fn popular_licenses_match_without_a_declaration() {
    let popular = [
        "MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "MPL-2.0", "ISC", "Unlicense", "CC0-1.0", "0BSD", "BSL-1.0", "Zlib", "EPL-2.0",
        "EUPL-1.2", "WTFPL",
    ];
    for id in popular {
        let found = match_text(&real_text(id), &[]);
        assert_eq!(found.iter().map(|item| item.0.as_str()).collect::<Vec<_>>(), vec![id], "{} matched {:?}", id, found);
    }
    for (id, family) in [("GPL-3.0-only", "GPL-3.0"), ("GPL-2.0-only", "GPL-2.0"), ("AGPL-3.0-only", "AGPL-3.0"), ("LGPL-2.1-only", "LGPL-2.1")] {
        assert_eq!(match_text(&real_text(id), &[])[0].0, family, "{}", id);
        assert_eq!(match_text(&real_text(id), &[id.replace("-only", "-or-later")])[0].0, id.replace("-only", "-or-later"));
    }
    assert_eq!(identifier("// SPDX-License-Identifier: MIT OR Apache-2.0\n").as_deref(), Some("MIT OR Apache-2.0"));
    assert!(match_text("Do whatever you want, I guess.", &[]).is_empty());
}

#[test]
fn files_holding_several_licenses_list_each() {
    let combined = format!("{}\n\n---\n\n{}", real_text("MPL-2.0"), real_text("Apache-2.0"));
    let ids: Vec<String> = match_text(&combined, &[]).into_iter().map(|found| found.0).collect();
    assert_eq!(ids, vec!["MPL-2.0".to_string(), "Apache-2.0".to_string()]);
    let pair = format!("{}\n\n{}", real_text("MIT"), real_text("ISC"));
    let mut ids: Vec<String> = match_text(&pair, &[]).into_iter().map(|found| found.0).collect();
    ids.sort();
    assert_eq!(ids, vec!["ISC".to_string(), "MIT".to_string()]);
}

#[test]
fn copyright_notices_drop_but_sentences_stay() {
    let words = super::words::words("Copyright Joyent, Inc.\ncopyright notice and this permission notice appear in all copies.\n(c) 2020 X\n");
    assert!(!words.contains(&"joyent".to_string()));
    assert!(words.contains(&"appear".to_string()) && words.contains(&"notice".to_string()));
}

#[test]
fn short_notices_name_their_license() {
    let apache = "Licensed under the Apache License, Version 2.0 (the \"License\");\n\
        you may not use this file except in compliance with the License.";
    assert_eq!(notice(apache), Some("Apache-2.0"));
    let gpl = "This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as \
        published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.";
    assert_eq!(notice(gpl), Some("GPL-3.0-or-later"));
    assert_eq!(notice("under the terms of the GNU Lesser General Public License version 2.1"), Some("LGPL-2.1-only"));
    assert_eq!(notice("Artwork: CC BY-SA 4.0"), Some("CC-BY-SA-4.0"));
    assert_eq!(notice("Released under the MIT License."), Some("MIT"));
    assert_eq!(notice("All rights reserved."), None);
}

#[test]
fn gpl_without_declaration_is_the_family() {
    let found = match_text(&text_of("GPL-3.0-only"), &[]).remove(0);
    assert_eq!(found.0, "GPL-3.0");
    assert_eq!(found.1, "GNU General Public License v3.0");
}

#[test]
fn manifests_declare_licenses() {
    assert_eq!(declared_license("package.json", r#"{"license":"AGPL-3.0-only"}"#).as_deref(), Some("AGPL-3.0-only"));
    assert_eq!(declared_license("package.json", r#"{"licenses":[{"type":"MIT"},{"type":"Apache-2.0"}]}"#).as_deref(), Some("MIT OR Apache-2.0"));
    let cargo = "[package]\nname = \"x\"\nlicense-file = \"L\"\nlicense = \"MIT OR Apache-2.0\"\n";
    assert_eq!(declared_license("Cargo.toml", cargo).as_deref(), Some("MIT OR Apache-2.0"));
    assert_eq!(declared_license("pyproject.toml", "[project]\nlicense = { text = \"BSD-3-Clause\" }\n").as_deref(), Some("BSD-3-Clause"));
    assert_eq!(declared_license("package.json", r#"{"license":"UNLICENSED"}"#), None);
}
