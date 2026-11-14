use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::parse::{Family, ImportRef};

const SCRIPT_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"];

pub enum Resolution {
    External,
    Files(Vec<String>),
    Unresolved,
}

#[derive(Clone, Default)]
struct TsPaths {
    base: String,
    patterns: Vec<(String, Vec<String>)>,
}

const ALIAS_MAP_DIR: &str = ".trebired/code-discipline/imports/";

pub struct Resolver {
    aliases: HashMap<String, HashMap<String, String>>,
    configs: HashMap<String, String>,
    dirs: HashMap<String, Vec<String>>,
    files: HashSet<String>,
    go_modules: Vec<(String, String)>,
    ts_cache: Mutex<HashMap<String, Option<TsPaths>>>,
}

pub fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

pub fn join(base: &str, relative: &str) -> String {
    let mut parts: Vec<&str> = if base.is_empty() { Vec::new() } else { base.split('/').collect() };
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn ancestors(dir: &str) -> Vec<String> {
    let mut out = vec![dir.to_string()];
    let mut current = dir.to_string();
    while !current.is_empty() {
        current = dir_of(&current).to_string();
        out.push(current.clone());
    }
    out
}

fn first_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(items) => items.iter().find_map(first_string),
        serde_json::Value::Object(map) => ["import", "default", "node", "require"].iter()
        .find_map(|key| map.get(*key).and_then(first_string))
        .or_else(|| map.values().find_map(first_string)),
        _ => None,
    }
}

fn alias_maps(configs: &HashMap<String, String>) -> HashMap<String, HashMap<String, String>> {
    let mut maps: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (path, text) in configs {
        let Some(position) = path.find(ALIAS_MAP_DIR) else { continue };
        if position != 0 && !path[..position].ends_with('/') {
            continue;
        }
        let root = path[..position].trim_end_matches('/').to_string();
        let Some(serde_json::Value::Object(entries)) = crate::jsonc::parse(text) else { continue };
        let map = maps.entry(root).or_default();
        for (alias, target) in entries {
            if let Some(target) = target.as_str() {
                map.insert(alias, target.to_string());
            }
        }
    }
    maps
}

fn match_pattern(pattern: &str, spec: &str) -> Option<String> {
    match pattern.split_once('*') {
        None => (pattern == spec).then(String::new),
        Some((prefix, suffix)) => {
            (spec.starts_with(prefix) && spec.ends_with(suffix) && spec.len() >= prefix.len() + suffix.len())
            .then(|| spec[prefix.len()..spec.len() - suffix.len()].to_string())
        }
    }
}

impl Resolver {
    pub fn new(paths: &[String], configs: HashMap<String, String>) -> Self {
        let mut dirs: HashMap<String, Vec<String>> = HashMap::new();
        for path in paths {
            dirs.entry(dir_of(path).to_string()).or_default().push(path.clone());
        }
        let go_modules = configs.iter().filter(|(path, _)| path.ends_with("go.mod") && path.rsplit('/').next() == Some("go.mod"))
        .filter_map(|(path, text)| {
                let module = text.lines().find_map(|line| line.trim().strip_prefix("module ").map(|name| name.trim().to_string()))?;
                Some((module, dir_of(path).to_string()))
        }).collect();
        let aliases = alias_maps(&configs);
        Resolver { aliases, configs, dirs, files: paths.iter().cloned().collect(), go_modules, ts_cache: Mutex::new(HashMap::new()) }
    }

    pub fn resolve(&self, family: Family, importer: &str, reference: &ImportRef) -> Resolution {
        match family {
            Family::Script => self.script(importer, &reference.spec),
            Family::Python => self.python(importer, reference),
            Family::Rust => self.rust(importer, reference),
            Family::Go => self.go(&reference.spec),
        }
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains(path)
    }

    fn script_file(&self, base: &str) -> Option<String> {
        if self.exists(base) {
            return Some(base.to_string());
        }
        for extension in SCRIPT_EXTENSIONS {
            let candidate = format!("{}{}", base, extension);
            if self.exists(&candidate) {
                return Some(candidate);
            }
        }
        for extension in SCRIPT_EXTENSIONS {
            let candidate = format!("{}/index{}", base, extension);
            if self.exists(&candidate) {
                return Some(candidate);
            }
        }
        let stem = [".js", ".jsx", ".mjs", ".cjs"].iter().find_map(|extension| base.strip_suffix(extension))?;
        [".ts", ".tsx", ".mts", ".cts"].iter().map(|extension| format!("{}{}", stem, extension)).find(|candidate| self.exists(candidate))
    }

    fn script(&self, importer: &str, spec: &str) -> Resolution {
        let dir = dir_of(importer);
        if spec.starts_with('.') {
            return self.script_file(&join(dir, spec)).map_or(Resolution::Unresolved, |file| Resolution::Files(vec![file]));
        }
        if let Some(file) = self.alias(dir, spec).or_else(|| self.package_imports(dir, spec)).or_else(|| self.ts_paths(dir, spec)) {
            return Resolution::Files(vec![file]);
        }
        if spec.starts_with('#') { Resolution::Unresolved } else { Resolution::External }
    }

    fn alias(&self, dir: &str, spec: &str) -> Option<String> {
        if !spec.starts_with('#') {
            return None;
        }
        ancestors(dir).into_iter().find_map(|root| {
                let target = self.aliases.get(&root)?.get(spec)?;
                self.script_file(&join(&root, target))
        })
    }

    fn package_imports(&self, dir: &str, spec: &str) -> Option<String> {
        if !spec.starts_with('#') {
            return None;
        }
        let package_dir = ancestors(dir).into_iter().find(|candidate| self.configs.contains_key(&join(candidate, "package.json")))?;
        let manifest = crate::jsonc::parse(self.configs.get(&join(&package_dir, "package.json"))?)?;
        let imports = manifest.get("imports")?.as_object()?;
        imports.iter().find_map(|(pattern, target)| {
                let captured = match_pattern(pattern, spec)?;
                let target = first_string(target)?.replace('*', &captured);
                self.script_file(&join(&package_dir, &target))
        })
    }

    fn ts_paths(&self, dir: &str, spec: &str) -> Option<String> {
        let config_dir = ancestors(dir).into_iter().find(|candidate| {
                self.configs.contains_key(&join(candidate, "tsconfig.json")) || self.configs.contains_key(&join(candidate, "jsconfig.json"))
        })?;
        let name = if self.configs.contains_key(&join(&config_dir, "tsconfig.json")) { "tsconfig.json" } else { "jsconfig.json" };
        let paths = self.load_ts(&join(&config_dir, name), 0)?;
        for (pattern, targets) in &paths.patterns {
            let Some(captured) = match_pattern(pattern, spec) else { continue };
            for target in targets {
                if let Some(file) = self.script_file(&join(&paths.base, &target.replace('*', &captured))) {
                    return Some(file);
                }
            }
        }
        None
    }

    fn load_ts(&self, path: &str, depth: usize) -> Option<TsPaths> {
        if let Some(cached) = self.ts_cache.lock().unwrap().get(path) {
            return cached.clone();
        }
        let loaded = if depth > 8 { None } else { self.read_ts(path, depth) };
        self.ts_cache.lock().unwrap().insert(path.to_string(), loaded.clone());
        loaded
    }

    fn read_ts(&self, path: &str, depth: usize) -> Option<TsPaths> {
        let config = crate::jsonc::parse(self.configs.get(path)?)?;
        let dir = dir_of(path);
        let mut merged = config.get("extends").and_then(|value| value.as_str()).filter(|value| value.starts_with('.'))
        .and_then(|parent| {
                let target = join(dir, parent);
                let target = if target.ends_with(".json") { target } else { format!("{}.json", target) };
                self.load_ts(&target, depth + 1)
        }).unwrap_or_default();
        let options = config.get("compilerOptions");
        if let Some(paths) = options.and_then(|options| options.get("paths")).and_then(|paths| paths.as_object()) {
            merged.patterns = paths.iter().map(|(pattern, targets)| {
                    let list = targets.as_array()
                    .map(|items| items.iter().filter_map(|item| item.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                    (pattern.clone(), list)
            }).collect();
            merged.base = dir.to_string();
        }
        if let Some(base_url) = options.and_then(|options| options.get("baseUrl")).and_then(|value| value.as_str()) {
            merged.base = join(dir, base_url);
        }
        Some(merged)
    }

    fn python_module(&self, base: &str, module: &str) -> Option<String> {
        let path = join(base, &module.replace('.', "/"));
        [format!("{}.py", path), format!("{}/__init__.py", path), format!("{}.pyi", path)].into_iter().find(|candidate| self.exists(candidate))
    }

    fn python(&self, importer: &str, reference: &ImportRef) -> Resolution {
        let spec = reference.spec.as_str();
        let dots = spec.chars().take_while(|character| *character == '.').count();
        let module = &spec[dots..];
        let roots: Vec<String> = if dots > 0 {
            let mut base = dir_of(importer).to_string();
            for _ in 1..dots {
                base = dir_of(&base).to_string();
            }
            vec![base]
        } else {
            let mut roots = ancestors(dir_of(importer));
            roots.push("src".to_string());
            roots
        };
        let mut files = Vec::new();
        for root in &roots {
            if !module.is_empty() {
                if let Some(file) = self.python_module(root, module) {
                    files.push(file);
                }
            }
            for binding in reference.bindings.iter().filter(|binding| reference.kind == "from" && binding.imported != "*") {
                let child = if module.is_empty() { binding.imported.clone() } else { format!("{}.{}", module, binding.imported) };
                if let Some(file) = self.python_module(root, &child) {
                    files.push(file);
                }
            }
            if !files.is_empty() {
                files.dedup();
                return Resolution::Files(files);
            }
        }
        if dots > 0 || roots.iter().any(|root| self.dirs.contains_key(&join(root, module.split('.').next().unwrap_or("")))) {
            Resolution::Unresolved
        } else {
            Resolution::External
        }
    }

    fn rust_module_dir(&self, file: &str) -> String {
        let name = file.rsplit('/').next().unwrap_or(file);
        if matches!(name, "mod.rs" | "lib.rs" | "main.rs") {
            return dir_of(file).to_string();
        }
        join(dir_of(file), name.trim_end_matches(".rs"))
    }

    fn rust_child(&self, dir: &str, name: &str) -> Option<String> {
        [join(dir, &format!("{}.rs", name)), join(dir, &format!("{}/mod.rs", name))].into_iter().find(|candidate| self.exists(candidate))
    }

    fn rust_crate_root(&self, file: &str) -> Option<String> {
        let manifest_dir = ancestors(dir_of(file)).into_iter().find(|dir| self.configs.contains_key(&join(dir, "Cargo.toml")))?;
        ["src/lib.rs", "src/main.rs"].iter().map(|root| join(&manifest_dir, root)).find(|root| self.exists(root))
    }

    fn rust(&self, importer: &str, reference: &ImportRef) -> Resolution {
        if reference.kind == "include" {
            let target = join(dir_of(importer), &reference.spec);
            return if self.exists(&target) { Resolution::Files(vec![target]) } else { Resolution::Unresolved };
        }
        if reference.kind == "mod" {
            return self.rust_child(
                &self.rust_module_dir(importer),
                &reference.spec
            ).map_or(Resolution::Unresolved, |file| Resolution::Files(vec![file]));
        }
        let segments: Vec<&str> = reference.spec.split("::").collect();
        let (mut module_file, rest) = match segments.first().copied() {
            Some("crate") => match self.rust_crate_root(importer) {
                Some(root) => (root, &segments[1..]),
                None => return Resolution::Unresolved,
            },
            Some("self") => (importer.to_string(), &segments[1..]),
            Some("super") => {
                let parent_dir = dir_of(&self.rust_module_dir(importer)).to_string();
                let parent = self.rust_child(dir_of(&parent_dir), parent_dir.rsplit('/').next().unwrap_or(""))
                .or_else(|| ["mod.rs", "lib.rs", "main.rs"].iter().map(|name| join(&parent_dir, name)).find(|path| self.exists(path)));
                match parent {
                    Some(parent) => (parent, &segments[1..]),
                    None => return Resolution::Unresolved,
                }
            }
            _ => return Resolution::External,
        };
        for segment in rest {
            match self.rust_child(&self.rust_module_dir(&module_file), segment) {
                Some(next) => module_file = next,
                None => break,
            }
        }
        if module_file == importer { Resolution::Unresolved } else { Resolution::Files(vec![module_file]) }
    }

    pub fn go_package_files(&self, dir: &str) -> Vec<String> {
        let mut files: Vec<String> = self.dirs.get(dir).map(|items| items.iter()
            .filter(|path| path.ends_with(".go") && !path.ends_with("_test.go")).cloned().collect()).unwrap_or_default();
        files.sort();
        files
    }

    fn go(&self, spec: &str) -> Resolution {
        for (module, root) in &self.go_modules {
            if spec == module || spec.starts_with(&format!("{}/", module)) {
                let relative = spec[module.len()..].trim_start_matches('/');
                let files = self.go_package_files(&join(root, relative));
                return if files.is_empty() { Resolution::Unresolved } else { Resolution::Files(files) };
            }
        }
        Resolution::External
    }
}
