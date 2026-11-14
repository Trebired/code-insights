use std::io::Read;
use std::path::PathBuf;

use crate::progress::Reporter;
use crate::scan::{scan_repository, ScanOptions};

const USAGE: &str = "usage: code-insights scan --git-dir <dir> [--ref <ref>] --out <file> [--phases a,b] \
[--max-parse-bytes n] [--history n] | code-insights detect --path <path> < content";

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|arg| arg == flag).and_then(|index| args.get(index + 1)).cloned()
}

fn number_of(args: &[String], flag: &str, fallback: usize) -> usize {
    value_of(args, flag).and_then(|value| value.parse().ok()).unwrap_or(fallback)
}

fn fail(code: &str, message: &str) -> i32 {
    crate::progress::emit_line(&serde_json::json!({ "type": "error", "code": code, "message": message }));
    1
}

fn run_scan(args: &[String]) -> i32 {
    let Some(git_dir) = value_of(args, "--git-dir") else { return fail("missing-git-dir", USAGE) };
    let Some(out) = value_of(args, "--out") else { return fail("missing-out", USAGE) };
    let options = ScanOptions {
        git_dir: PathBuf::from(git_dir),
        history: number_of(args, "--history", 1000),
        max_function_edges: number_of(args, "--max-function-edges", 60_000),
        max_function_nodes: number_of(args, "--max-function-nodes", 20_000),
        max_import_edges: number_of(args, "--max-import-edges", 50_000),
        max_parse_bytes: number_of(args, "--max-parse-bytes", 1_048_576),
        max_read_bytes: number_of(args, "--max-read-bytes", 8_388_608),
        phases: crate::scan::parse_phases(value_of(args, "--phases").as_deref()),
        reference: value_of(args, "--ref").unwrap_or_else(|| "HEAD".to_string()),
    };
    let reporter = Reporter::new(&options.phases);
    match scan_repository(&options, &reporter) {
        Ok(result) => match std::fs::write(&out, serde_json::to_vec(&result).unwrap_or_default()) {
            Ok(()) => {
                reporter.done();
                0
            }
            Err(error) => fail("write-failed", &error.to_string()),
        },
        Err(error) => fail(&error.code, &error.message),
    }
}

fn run_detect(args: &[String]) -> i32 {
    let Some(path) = value_of(args, "--path") else { return fail("missing-path", USAGE) };
    let mut content = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut content);
    let text = String::from_utf8_lossy(&content);
    let detected = crate::lang::detect(&path, &text).map(crate::lang::describe);
    println!("{}", serde_json::json!({ "type": "language", "language": detected }));
    0
}

pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("scan") => run_scan(&args[2..]),
        Some("detect") => run_detect(&args[2..]),
        Some("--version") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => fail("usage", USAGE),
    }
}
