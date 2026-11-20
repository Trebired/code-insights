# Changelog

All notable changes to `@trebired/code-insights` will be documented here.

This project follows semantic versioning once published.

## 0.2.0

- Hotspot rows use the `card-03` container of the `@trebired/configs` frontend preset (0.12.0), which replaced `line-card`, and the list is `tbf-column tbf-divided`, which replaced `line-list`.
- Accepts `@trebired/frontend` 32 as a peer. 32 moved the card look into configuration containers and kept the component API, so the package renders the same markup.
- Added the `license` phase. It matched root license files against the 727 licenses of the SPDX License List, read `SPDX-License-Identifier` lines, short license notices and files that hold several licenses, and read the license declared in `package.json`, `composer.json`, `Cargo.toml` and `pyproject.toml`. The result is `license` in `CodeInsightsResult`, typed as `LicenseInfo` and `LicenseMatch`, with each license's steward and SPDX page URLs, the manifest that declared a license, and `agreement` between the declaration and the license files.
- `SCAN_PHASES` lists `license` after `listing`.

## 0.1.3

- Hotspot paths show a tooltip only when they are cut off. Every path carried a native `title`, so short paths got a tooltip repeating text already on screen. The peer and dev range for `@trebired/frontend` is now `^31.0.8`, which added overflow-only tooltips.

## 0.1.2

- `hotspot_list` puts its rows in a `tbfm-line-list`, so the hairlines between rows come from the frontend preset.
- The code map's Files and Functions switches are primary buttons.

## 0.1.1

- Fixed the code map laying out blind when it is bound inside a hidden tab or panel. It now lays out and fits again the first time it becomes visible, and refits on resize until the user pans or zooms.

## 0.1.0

- Added the native engine: languages with Linguist's rules, the file tree, a file import graph and a function call graph for TypeScript, JavaScript, Python, Rust and Go, and hotspots from git history, streamed as live progress per phase.
- Added `scanRepository`, `detectLanguage` and `buildCodeMapData` in `@trebired/code-insights/server`.
- Added the `code_map` and `hotspot_list` components in `@trebired/code-insights/react`, and `bindCodeMaps` and `reloadCodeMaps` in `@trebired/code-insights/client`.
