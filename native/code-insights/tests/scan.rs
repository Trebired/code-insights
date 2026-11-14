use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("code-insights-test-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (path, content) in files {
        let target = dir.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, content).unwrap();
    }
    let git = |args: &[&str]| {
        let status = Command::new("git").args(args).current_dir(&dir)
        .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t")
        .status().unwrap();
        assert!(status.success());
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "one"]);
    dir
}

fn scan(dir: &Path) -> (serde_json::Value, Vec<serde_json::Value>) {
    let out = dir.join("result.json");
    let output = Command::new(env!("CARGO_BIN_EXE_code-insights"))
    .args(["scan", "--git-dir", dir.to_str().unwrap(), "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let events = String::from_utf8_lossy(&output.stdout).lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    (serde_json::from_slice(&fs::read(out).unwrap()).unwrap(), events)
}

fn import_edges(result: &serde_json::Value) -> HashSet<(String, String)> {
    let nodes = result["imports"]["nodes"].as_array().unwrap();
    result["imports"]["edges"].as_array().unwrap().iter().map(|edge| {
            let from = nodes[edge["from"].as_u64().unwrap() as usize].as_str().unwrap().to_string();
            let to = nodes[edge["to"].as_u64().unwrap() as usize].as_str().unwrap().to_string();
            (from, to)
    }).collect()
}

fn call_edges(result: &serde_json::Value) -> HashSet<(String, String)> {
    let nodes = result["functions"]["nodes"].as_array().unwrap();
    let label = |id: u64| {
        let node = &nodes[id as usize];
        let file = result["functions"]["files"][node["file"].as_u64().unwrap() as usize].as_str().unwrap();
        format!("{}#{}", file, node["name"].as_str().unwrap())
    };
    result["functions"]["edges"].as_array().unwrap().iter()
    .map(|edge| (label(edge["from"].as_u64().unwrap()), label(edge["to"].as_u64().unwrap()))).collect()
}

fn has(edges: &HashSet<(String, String)>, from: &str, to: &str) -> bool {
    edges.contains(&(from.to_string(), to.to_string()))
}

#[test]
fn typescript_imports_aliases_and_calls() {
    let dir = fixture("ts", &[
            ("tsconfig.json", "{ // aliases\n \"extends\": \"./config/paths.json\" }"),
            ("config/paths.json", "{ \"compilerOptions\": { \"baseUrl\": \"..\", \"paths\": { \"@lib/*\": [\"src/lib/*\"] } } }"),
            (".trebired/code-discipline/imports/1.json", "{ \"#util\": \"./src/lib/util.ts\" }"),
            ("src/lib/util.ts", "export function helper() { return 1; }\nexport const twice = () => helper() * 2;\n"),
            ("src/lib/index.ts", "export * from './util';\n"),
            ("src/app.ts", "import { helper } from '#util';\nimport { twice as t } from '@lib/util';\nimport * as lib from './lib';\n\
                export function main() { helper(); t(); lib.helper(); }\n"),
            ("src/esm.ts", "import { main } from './app.js';\nclass Runner { run() { this.go(); main(); } go() {} }\n"),
            ("node_modules/x/index.js", "module.exports = 1;"),
    ]);
    let (result, events) = scan(&dir);
    let imports = import_edges(&result);
    assert!(has(&imports, "src/app.ts", "src/lib/util.ts"), "{:?}", imports);
    assert!(has(&imports, "src/app.ts", "src/lib/index.ts"));
    assert!(has(&imports, "src/lib/index.ts", "src/lib/util.ts"));
    assert!(has(&imports, "src/esm.ts", "src/app.ts"));
    let calls = call_edges(&result);
    assert!(has(&calls, "src/app.ts#main", "src/lib/util.ts#helper"), "{:?}", calls);
    assert!(has(&calls, "src/app.ts#main", "src/lib/util.ts#twice"));
    assert!(has(&calls, "src/lib/util.ts#twice", "src/lib/util.ts#helper"));
    assert!(has(&calls, "src/esm.ts#run", "src/esm.ts#go"));
    assert!(has(&calls, "src/esm.ts#run", "src/app.ts#main"));
    assert!(result["skipped"]["vendored"].as_u64().unwrap() >= 1);
    let phases: Vec<&str> = events.iter()
    .filter(|event| event["type"] == "phase" && event["state"] == "start")
    .map(|event| event["phase"].as_str().unwrap())
    .collect();
    assert_eq!(phases, vec!["listing", "languages", "tree", "imports", "functions", "hotspots"]);
    assert_eq!(events.last().unwrap()["type"], "done");
}

#[test]
fn python_relative_and_package_imports() {
    let dir = fixture("py", &[
            ("pkg/__init__.py", ""),
            ("pkg/core.py", concat!(
                    "def compute():\n    return 1\n\n",
                    "class Engine:\n    def run(self):\n        return self.step()\n",
                    "    def step(self):\n        return compute()\n",
            )),
            ("pkg/api.py", concat!(
                    "from .core import compute\nfrom . import core\n\n",
                    "def handler():\n    compute()\n    core.compute()\n",
            )),
            ("main.py", "import pkg.api as api\nimport requests\n\ndef start():\n    api.handler()\n"),
    ]);
    let (result, _) = scan(&dir);
    let imports = import_edges(&result);
    assert!(has(&imports, "pkg/api.py", "pkg/core.py"), "{:?}", imports);
    assert!(has(&imports, "main.py", "pkg/api.py"));
    assert!(result["imports"]["external"].as_u64().unwrap() >= 1);
    let calls = call_edges(&result);
    assert!(has(&calls, "pkg/api.py#handler", "pkg/core.py#compute"), "{:?}", calls);
    assert!(has(&calls, "main.py#start", "pkg/api.py#handler"));
    assert!(has(&calls, "pkg/core.py#run", "pkg/core.py#step"));
}

#[test]
fn rust_modules_uses_and_includes() {
    let dir = fixture("rs", &[
            ("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n"),
            ("src/lib.rs", "mod parser;\npub mod util;\ninclude!(\"extra.rs\");\nuse crate::util::tools::shout;\n\
                pub fn run() { parser::parse(); shout(); Thing::new(); }\n"),
            ("src/extra.rs", "pub struct Thing;\nimpl Thing { pub fn new() -> Self { Self::make() } fn make() -> Self { Thing } }\n"),
            ("src/parser.rs", "pub fn parse() {}\n"),
            ("src/util/mod.rs", "pub mod tools;\n"),
            ("src/util/tools.rs", "use super::super::parser;\npub fn shout() { parser::parse(); }\n"),
    ]);
    let (result, _) = scan(&dir);
    let imports = import_edges(&result);
    assert!(has(&imports, "src/lib.rs", "src/parser.rs"), "{:?}", imports);
    assert!(has(&imports, "src/lib.rs", "src/util/mod.rs"));
    assert!(has(&imports, "src/lib.rs", "src/extra.rs"));
    assert!(has(&imports, "src/util/mod.rs", "src/util/tools.rs"));
    assert!(has(&imports, "src/lib.rs", "src/util/tools.rs"));
    let calls = call_edges(&result);
    assert!(has(&calls, "src/lib.rs#run", "src/util/tools.rs#shout"), "{:?}", calls);
    assert!(has(&calls, "src/extra.rs#new", "src/extra.rs#make"));
}

#[test]
fn go_module_packages() {
    let dir = fixture("go", &[
            ("go.mod", "module example.com/app\n\ngo 1.22\n"),
            ("store/store.go", "package store\n\ntype Store struct{}\n\nfunc New() *Store { return &Store{} }\n\n\
                func (s *Store) Save() { s.flush() }\n\nfunc (s *Store) flush() {}\n"),
            ("store/helpers.go", "package store\n\nfunc normalize() {}\n"),
            ("main.go", "package main\n\nimport (\n\t\"fmt\"\n\tdb \"example.com/app/store\"\n)\n\n\
                func main() { db.New(); fmt.Println(1); boot() }\n\nfunc boot() {}\n"),
    ]);
    let (result, _) = scan(&dir);
    let imports = import_edges(&result);
    assert!(has(&imports, "main.go", "store/store.go"), "{:?}", imports);
    assert!(has(&imports, "main.go", "store/helpers.go"));
    let calls = call_edges(&result);
    assert!(has(&calls, "main.go#main", "store/store.go#New"), "{:?}", calls);
    assert!(has(&calls, "main.go#main", "main.go#boot"));
    assert!(has(&calls, "store/store.go#Save", "store/store.go#flush"));
}

#[test]
fn languages_tree_binary_and_generated() {
    let dir = fixture("lang", &[
            ("README.md", "# Demo\n"),
            ("src/app.ts", "const a = 1;\n\nexport { a };\n"),
            ("src/gen.go", "// Code generated by tool. DO NOT EDIT.\npackage gen\n"),
            ("assets/logo.bin", "\u{0}\u{1}\u{2}binary"),
            ("bun.lock", "{}"),
    ]);
    let (result, _) = scan(&dir);
    let languages: Vec<&str> = result["languages"].as_array().unwrap().iter().map(|item| item["name"].as_str().unwrap()).collect();
    assert!(languages.contains(&"TypeScript"), "{:?}", languages);
    assert!(!languages.contains(&"Go"));
    assert_eq!(result["skipped"]["binary"], 1);
    assert!(result["skipped"]["generated"].as_u64().unwrap() >= 2);
    let typescript = result["languages"].as_array().unwrap().iter().find(|item| item["name"] == "TypeScript").unwrap();
    assert_eq!(typescript["lines"]["total"], 3);
    assert_eq!(typescript["lines"]["code"], 2);
    assert!(result["tree"]["ascii"].as_str().unwrap().starts_with("├── assets"));
}

#[test]
fn empty_repository_and_bad_path() {
    let dir = std::env::temp_dir().join(format!("code-insights-test-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    assert!(Command::new("git").args(["init", "-q"]).current_dir(&dir).status().unwrap().success());
    let (result, _) = scan(&dir);
    assert_eq!(result["empty"], true);
    let output = Command::new(env!("CARGO_BIN_EXE_code-insights")).args([
            "scan",
            "--git-dir",
            "/nonexistent/repo",
            "--out",
            "/dev/null"
    ]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not-a-repository"));
}
