# Contributing

Thanks for helping improve `@trebired/code-insights`.

## Development Setup

```sh
bun install
```

The package is authored in TypeScript and Rust. The TypeScript part is published from `dist`; the engine is built with Cargo into `native/<platform>/code-insights`. Generated outputs, Cargo targets, package tarballs, temp folders, logs and caches stay out of Git.

## Common Commands

```sh
bun install --frozen-lockfile
bunx @trebired/code-discipline check
bun run typecheck
bun run build:native
bun run verify:native
bun run build
bun run verify:pack
bun run verify:scan
bun run verify:map
```

`verify:native` runs the engine's Cargo tests against real git fixtures in every supported language. `verify:scan` drives the built API: progress order, results, phases, cancelling and language detection. `verify:map` checks the code map data, the server-rendered components and the client in happy-dom.

## Pull Request Checklist

- Add a fixture case for every resolution rule you change.
- Keep public API changes intentional and documented in `README.md`.
- Run Code Discipline, typecheck, the Cargo tests, build, and every verification script.
- Update `CHANGELOG.md` under the current version or a new version section.
- Do not commit `dist`, Cargo targets, engine binaries, package tarballs, temp folders, logs or caches.

## Code Discipline

- Keep the config at `.trebired/code-discipline/config.ts`.
- Keep `@trebired/code-discipline` in `devDependencies`.
- `native/vendor` holds third-party code and is excluded; nothing else is.

## Design Principles

- Keep the scan out of the application's process: the engine is a separate binary that streams NDJSON progress and writes its result to a file.
- Draw an edge only when the engine can name its target; count everything else as unresolved.
- Keep the root entry browser-safe: it exports constants and types only.
