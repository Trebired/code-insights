# Changelog

All notable changes to `@trebired/code-insights` will be documented here.

This project follows semantic versioning once published.

## 0.1.2

- `hotspot_list` puts its rows in a `tbfm-line-list`, so the hairlines between rows come from the frontend preset.
- The code map's Files and Functions switches are primary buttons.

## 0.1.1

- Fixed the code map laying out blind when it is bound inside a hidden tab or panel. It now lays out and fits again the first time it becomes visible, and refits on resize until the user pans or zooms.

## 0.1.0

- Added the native engine: languages with Linguist's rules, the file tree, a file import graph and a function call graph for TypeScript, JavaScript, Python, Rust and Go, and hotspots from git history, streamed as live progress per phase.
- Added `scanRepository`, `detectLanguage` and `buildCodeMapData` in `@trebired/code-insights/server`.
- Added the `code_map` and `hotspot_list` components in `@trebired/code-insights/react`, and `bindCodeMaps` and `reloadCodeMaps` in `@trebired/code-insights/client`.
