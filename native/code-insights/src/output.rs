use serde::Serialize;
use std::collections::BTreeMap;

pub const RESULT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Lines {
    pub total: u64,
    pub content: u64,
    pub code: u64,
}

impl Lines {
    pub fn add(&mut self, other: &Lines) {
        self.total += other.total;
        self.content += other.content;
        self.code += other.code;
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Totals {
    pub bytes: u64,
    pub count: u64,
    pub lines: Lines,
}

#[derive(Clone, Debug, Serialize)]
pub struct LanguageTotal {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub color: String,
    pub bytes: u64,
    pub count: u64,
    pub percent: f64,
    pub lines: Lines,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileRecord {
    pub path: String,
    pub language: Option<String>,
    pub bytes: u64,
    pub lines: Lines,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Skipped {
    pub binary: u64,
    pub documentation: u64,
    pub generated: u64,
    pub vendored: u64,
    pub unparsed: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TreeNode {
    pub kind: &'static str,
    pub language: Option<String>,
    pub mode: String,
    pub name: String,
    pub object: String,
    pub path: String,
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TreeOutput {
    pub ascii: String,
    pub nested: Vec<TreeNode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportEdge {
    pub from: usize,
    pub to: usize,
    pub kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnresolvedImport {
    pub from: usize,
    pub spec: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ImportGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<ImportEdge>,
    pub external: u64,
    pub unresolved: u64,
    pub unresolved_samples: Vec<UnresolvedImport>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionNode {
    pub file: usize,
    pub name: String,
    pub kind: &'static str,
    pub owner: Option<String>,
    pub line: usize,
    pub end_line: usize,
    pub exported: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CallEdge {
    pub from: usize,
    pub to: usize,
    pub confidence: &'static str,
    pub count: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FunctionGraph {
    pub files: Vec<String>,
    pub nodes: Vec<FunctionNode>,
    pub edges: Vec<CallEdge>,
    pub unresolved: u64,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hotspot {
    pub path: String,
    pub language: Option<String>,
    pub lines: u64,
    pub commits: u64,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LicenseMatch {
    pub id: String,
    pub name: String,
    pub osi_approved: bool,
    pub source: &'static str,
    pub path: Option<String>,
    pub confidence: f64,
    pub url: Option<String>,
    pub spdx_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LicenseOutput {
    pub licenses: Vec<LicenseMatch>,
    pub declared: Option<String>,
    pub declared_path: Option<String>,
    pub agreement: &'static str,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ScanResult {
    pub version: u32,
    pub empty: bool,
    pub commit: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub scanned_at: String,
    pub totals: Totals,
    pub unknown: Totals,
    pub skipped: Skipped,
    pub languages: Vec<LanguageTotal>,
    pub files: Vec<FileRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<TreeOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<ImportGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<FunctionGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspots: Option<Vec<Hotspot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<LicenseOutput>,
    pub timings_ms: BTreeMap<String, u64>,
}
