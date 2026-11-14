use std::collections::HashMap;

use crate::output::{FileRecord, Hotspot};

pub fn build(files: &[FileRecord], churn: &HashMap<String, u64>, programming: &dyn Fn(&str) -> bool, limit: usize) -> Vec<Hotspot> {
    let mut ranked: Vec<(f64, &FileRecord, u64)> = files.iter()
    .filter(|file| file.language.as_deref().map_or(false, programming))
    .filter_map(|file| {
            let commits = *churn.get(&file.path)?;
            (commits > 0 && file.lines.code > 0).then(|| ((commits * file.lines.code) as f64, file, commits))
    }).collect();
    ranked.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(std::cmp::Ordering::Equal).then(left.1.path.cmp(&right.1.path)));
    let top = ranked.first().map(|item| item.0).unwrap_or(1.0).max(1.0);
    ranked.into_iter().take(limit).map(|(raw, file, commits)| Hotspot {
            path: file.path.clone(), language: file.language.clone(), lines: file.lines.code, commits,
            score: ((raw / top) * 1000.0).round() / 10.0,
    }).collect()
}
