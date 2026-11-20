use std::fmt::Write as _;
use std::path::PathBuf;

#[path = "src/license/words.rs"]
mod license_words;

const DEPRECATED: &[&str] = &[
    "AGPL-1.0", "AGPL-3.0", "BSD-2-Clause-FreeBSD", "BSD-2-Clause-NetBSD", "bzip2-1.0.5", "eCos-2.0", "GFDL-1.1", "GFDL-1.2",
    "GFDL-1.3", "GPL-1.0", "GPL-1.0+", "GPL-2.0", "GPL-2.0+", "GPL-2.0-with-autoconf-exception", "GPL-2.0-with-bison-exception",
    "GPL-2.0-with-classpath-exception", "GPL-2.0-with-font-exception", "GPL-2.0-with-GCC-exception", "GPL-3.0", "GPL-3.0+",
    "GPL-3.0-with-autoconf-exception", "GPL-3.0-with-GCC-exception", "LGPL-2.0", "LGPL-2.0+", "LGPL-2.1", "LGPL-2.1+", "LGPL-3.0",
    "LGPL-3.0+", "Net-SNMP", "Nunit", "StandardML-NJ", "wxWindows",
];

fn field(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ").trim().to_string()
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source = manifest.join("../vendor/spdx-license-list/spdx-full.json");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed=src/license/words.rs");
    let raw = std::fs::read(&source).expect("vendored SPDX license list");
    let list: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&raw).expect("valid SPDX license list");
    let mut out = String::new();
    for (id, entry) in &list {
        let name = field(entry["name"].as_str().unwrap_or(id));
        let osi = entry["osiApproved"].as_bool().unwrap_or(false);
        let deprecated = DEPRECATED.contains(&id.as_str());
        let url = field(entry["url"].as_str().unwrap_or(""));
        let words = license_words::words(entry["licenseText"].as_str().unwrap_or(""));
        let _ = writeln!(out, "{}\t{}\t{}\t{}\t{}\t{}", field(id), name, u8::from(osi), u8::from(deprecated), url, words.join(" "));
    }
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out_dir.join("licenses.tsv"), out).expect("write license index");
}
