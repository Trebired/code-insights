use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::scan::ScanError;

#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub kind: String,
    pub mode: String,
    pub object: String,
    pub path: String,
    pub size: Option<u64>,
}

pub struct Git {
    dir: PathBuf,
}

impl Git {
    pub fn open(dir: &Path) -> Result<Self, ScanError> {
        let dotted = dir.join(".git");
        let git = Git { dir: if dotted.exists() { dotted } else { dir.to_path_buf() } };
        git.output(&["rev-parse", "--git-dir"]).map_err(|error| ScanError::new("not-a-repository", &error.message))?;
        Ok(git)
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git");
        command.arg("--git-dir").arg(&self.dir).env("GIT_TERMINAL_PROMPT", "0");
        command
    }

    fn output(&self, args: &[&str]) -> Result<Vec<u8>, ScanError> {
        let output = self.command().args(args).stdin(Stdio::null()).output()
        .map_err(|error| ScanError::new("git-unavailable", &error.to_string()))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(ScanError::new("git-failed", &message));
        }
        Ok(output.stdout)
    }

    pub fn resolve_commit(&self, reference: &str) -> Result<Option<String>, ScanError> {
        let target = format!("{}^{{commit}}", if reference.is_empty() { "HEAD" } else { reference });
        let output = self.command().args(["rev-parse", "--verify", "--quiet", &target]).stdin(Stdio::null()).output()
        .map_err(|error| ScanError::new("git-unavailable", &error.to_string()))?;
        if output.status.success() {
            return Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()));
        }
        if !self.output(&["rev-list", "-n", "1", "--all"])?.is_empty() {
            return Err(ScanError::new("ref-not-found", &format!("ref not found: {}", reference)));
        }
        Ok(None)
    }

    pub fn list_tree(&self, commit: &str) -> Result<Vec<TreeEntry>, ScanError> {
        let raw = self.output(&["ls-tree", "-r", "-t", "-z", "-l", "--full-tree", commit])?;
        Ok(raw.split(|byte| *byte == 0).filter(|item| !item.is_empty()).filter_map(parse_tree_line).collect())
    }

    pub fn read_blobs<F: FnMut(usize, Vec<u8>)>(&self, objects: &[String], mut on_blob: F) -> Result<(), ScanError> {
        let mut child = self.command().args(["cat-file", "--batch"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        .map_err(|error| ScanError::new("git-unavailable", &error.to_string()))?;
        let mut stdin = child.stdin.take().expect("stdin piped");
        let list: Vec<String> = objects.to_vec();
        let writer = std::thread::spawn(move || {
                for object in list {
                    if writeln!(stdin, "{}", object).is_err() {
                        break;
                    }
                }
        });
        let mut reader = BufReader::with_capacity(1 << 20, child.stdout.take().expect("stdout piped"));
        for index in 0..objects.len() {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                break;
            }
            let size: usize = header.trim_end().rsplit(' ').next().and_then(|value| value.parse().ok()).unwrap_or(0);
            if header.trim_end().ends_with("missing") {
                on_blob(index, Vec::new());
                continue;
            }
            let mut content = vec![0u8; size];
            reader.read_exact(&mut content).map_err(|error| ScanError::new("git-read-failed", &error.to_string()))?;
            let mut newline = [0u8; 1];
            let _ = reader.read_exact(&mut newline);
            on_blob(index, content);
        }
        let _ = writer.join();
        let _ = child.wait();
        Ok(())
    }

    pub fn churn(&self, commit: &str, limit: usize) -> Result<HashMap<String, u64>, ScanError> {
        let limit_arg = format!("-n{}", limit.max(1));
        let raw = self.output(&["log", &limit_arg, "--no-renames", "--format=%x1e", "--name-only", commit])?;
        let mut counts: HashMap<String, u64> = HashMap::new();
        for chunk in String::from_utf8_lossy(&raw).split('\u{1e}') {
            let mut seen = std::collections::HashSet::new();
            for line in chunk.lines().map(str::trim).filter(|line| !line.is_empty()) {
                if seen.insert(line.to_string()) {
                    *counts.entry(line.to_string()).or_insert(0) += 1;
                }
            }
        }
        Ok(counts)
    }
}

fn parse_tree_line(raw: &[u8]) -> Option<TreeEntry> {
    let text = String::from_utf8_lossy(raw);
    let (meta, path) = text.split_once('\t')?;
    let mut parts = meta.split_whitespace();
    let mode = parts.next()?.to_string();
    let kind = parts.next()?.to_string();
    let object = parts.next()?.to_string();
    let size = parts.next().and_then(|value| value.parse().ok());
    Some(TreeEntry { kind, mode, object, path: path.to_string(), size })
}
