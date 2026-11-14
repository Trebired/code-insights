# @trebired/code-insights

Repository insights for Bun applications, scanned by a native engine that reads a git repository directly: languages, the file tree, a file import graph, a function call graph and hotspots, with live progress, plus an interactive code map and hotspot list for `@trebired/frontend`.

The package owns reading the repository at a ref, language detection, line counts, vendored and generated file rules, the tree, import resolution, call resolution, churn, the progress stream, the result format, the code map data, and the code map and hotspot interface. The application owns where repositories live, when a scan runs, where results are stored, who may see them, and the routes that serve them. The package does not own a job queue, storage, permissions, or any HTTP route.

## Install

Runtime support: Bun 1+.

```sh
bun i @trebired/code-insights
```

## Quick Start

```ts
import { buildCodeMapData, scanRepository } from "@trebired/code-insights/server";

const result = await scanRepository({
  gitDir: "/srv/repositories/app.git",
  ref: "main",
  onProgress: ({ phase, percent }) => broadcast("scan", { phase, percent }),
});

const files = buildCodeMapData(result, "file");
const functions = buildCodeMapData(result, "function");
```

```tsx
import { code_map, hotspot_list } from "@trebired/code-insights/react";

code_map({ dataUrl: "/repository/insights/graph", hrefTemplate: "/repository/edit?path={path}&line={line}", lang });
hotspot_list({ hotspots: result.hotspots, hrefFor: (path) => `/repository/edit?path=${encodeURIComponent(path)}`, lang });
```

```ts
import { bindCodeMaps } from "@trebired/code-insights/client";

bindCodeMaps(document);
```

## Concepts

### Engine

The scan runs in a separate native process, not in the application's process. It reads the ref with the `git` command line, streams every blob through one `git cat-file --batch` process, and parses files in parallel. A crash, a huge repository or a cancelled scan never takes the application down, and the scan never blocks its event loop.

### Phases

A scan runs these phases in order. Each one reports `{ phase, done, total, percent }` while it runs, and `percent` is the share of the whole scan, weighted by phase.

| Phase | Result |
| --- | --- |
| `listing` | the commit and every tree entry |
| `languages` | `languages`, `files`, `totals`, `unknown`, `skipped` |
| `tree` | `tree.nested` and `tree.ascii` |
| `imports` | `imports`: files as nodes, resolved imports as edges |
| `functions` | `functions`: definitions as nodes, resolved calls as edges |
| `hotspots` | `hotspots`: code lines times commits that touched the file |

`phases` limits a scan to the phases it names. `listing` always runs, `functions` brings `imports` with it, and `imports` and `hotspots` bring `languages`.

### Languages

Detection follows GitHub Linguist's rules: file names, extensions, shebangs, heuristics and the classifier, with Linguist's colours and language types. Files are left out of the totals when they are binary, vendored (`node_modules`, `dist`, `vendor` and the rest of Linguist's list), generated (lockfiles, minified files, source maps, a generated header) or documentation. `.gitattributes` overrides all of these with `linguist-vendored`, `linguist-generated`, `linguist-documentation` and `linguist-language`.

### Import Resolution

Imports resolve to files in the same repository. Package imports count as external, and imports that point inside the repository but match no file count as unresolved, with up to 50 samples in `imports.unresolved_samples`.

- TypeScript and JavaScript: relative paths, extensions and `index` files, `.js` specifiers that name a `.ts` file, tsconfig `paths` and `baseUrl` through `extends`, package.json `imports`, and Code Discipline alias maps in `.trebired/code-discipline/imports/*.json`.
- Python: relative imports, packages and modules from the importing file's directory upwards and from `src`.
- Rust: `mod` declarations, `#[path]` modules, `include!`, and `use` paths starting with `crate`, `self` or `super`.
- Go: import paths under a `go.mod` module, to every non-test file of the package.

### Call Resolution

A call becomes an edge only when the engine can name its target: a function in the same file, an imported function, a method on `this` or `self`, a method of a type through `Type::method` or `Self::method`, a Go receiver method or a function of the same Go package. Anything else counts as unresolved and draws no edge. Each edge carries how it was resolved (`local`, `import`, `member` or `package`) and how often the call appears.

### Code Map

`buildCodeMapData(result, level)` turns either graph into nodes grouped by folder, choosing a folder depth that gives a readable number of groups. The map starts with every folder collapsed into one node. Clicking a folder opens it, clicking a file or function highlights what it links to and what links to it, and clicking it again opens it through `hrefTemplate`. A cancellable `code-map:open` event fires first, so the application can take over the navigation.

## Configuration

### Limits

`scanRepository` takes `limits`:

| Option | Default | Effect |
| --- | --- | --- |
| `maxReadBytes` | 8 MB | larger files are counted by size only |
| `maxParseBytes` | 1 MB | larger files are not parsed for imports or calls |
| `maxImportEdges` | 50 000 | further edges are dropped and `imports.truncated` is set |
| `maxFunctionNodes` | 20 000 | further definitions are dropped and `functions.truncated` is set |
| `maxFunctionEdges` | 60 000 | the most frequent calls are kept |
| `historyCommits` | 1 000 | commits read for hotspots |

`buildCodeMapData` takes `maxNodes` (default 4 000) and keeps the most connected nodes of a larger graph.

## Runtime

### Engine Binary

The package ships the engine for Linux x64 and arm64 with glibc, in `native/<platform>/code-insights`. `TREBIRED_CODE_INSIGHTS_BINARY` points the package at another build. Other platforms fail with `code-insights-binary-missing`.

### Errors and Cancelling

Failures reject with `CodeInsightsError` and a `code`: `not-a-repository`, `ref-not-found`, `git-unavailable`, `aborted`, `engine-failed` or `result-version-mismatch`. Passing an `AbortSignal` kills the engine process.

### Logging

The package logs through `@trebired/logger-adapter` under `trebired.code-insights` and stays silent unless the application passes a logger.

### Third-Party Code

The engine contains hyperpolyglot (MIT OR Apache-2.0), vendored with its detectors made public and shipped with its licence in `native/vendor/hyperpolyglot/LICENSE-MIT`, which carries GitHub Linguist's language data (MIT), and Tree-sitter with its TypeScript, JavaScript, Python, Rust and Go grammars (MIT). The client uses d3-force (ISC).

## Public API

| Entry | Exports |
| --- | --- |
| `@trebired/code-insights` | `PACKAGE_NAME`, `RESULT_VERSION`, `SCAN_PHASES` and every result type |
| `@trebired/code-insights/server` | `scanRepository`, `detectLanguage`, `buildCodeMapData`, `resolveBinary`, `CodeInsightsError`, `BINARY_ENV` |
| `@trebired/code-insights/react` | `code_map`, `hotspot_list` |
| `@trebired/code-insights/client` | `bindCodeMaps`, `bindCodeMap`, `reloadCodeMaps` |

`detectLanguage({ path, content })` detects one file's language with the same rules as a scan. `reloadCodeMaps(scope)` fetches every bound map's data again at its current level, for after a rescan.

## What It Does Not Do

This package does not:

- queue, schedule, store or cache scans;
- serve routes or check who may see a repository;
- run on macOS, Windows or musl Linux without a custom engine build;
- resolve calls through variables, interfaces, callbacks or dynamic dispatch;
- parse languages other than TypeScript, JavaScript, Python, Rust and Go for imports and calls;
- count vendored, generated, binary or documentation files in language totals.
