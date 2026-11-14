import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveLogger } from "@package/logger-adapter";

import { createRepository, importDist, SAMPLE_FILES } from "./fixtures.mjs";

const { CodeInsightsError, detectLanguage, scanRepository } = await importDist("server/index.js");

const log = resolveLogger({ source: "@trebired/code-insights" });
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const tempRoot = path.join(rootDir, ".tmp", "verify-scan");

async function verifyProgressAndResult(dir) {
  const progress = [];
  const phases = [];
  const result = await scanRepository({ gitDir: dir, onPhase: (event) => phases.push(event), onProgress: (event) => progress.push(event) });
  assert.equal(result.version, 1);
  assert.equal(result.totals.count, 6);
  const started = phases.filter((event) => event.state === "start").map((event) => event.phase);
  assert.deepEqual(started, ["listing", "languages", "tree", "imports", "functions", "hotspots"]);
  for (let index = 1; index < progress.length; index += 1) {
    assert.ok(progress[index].percent >= progress[index - 1].percent, "progress never goes backwards");
  }
  assert.equal(progress.at(-1).percent, 100);
  const edges = result.imports.edges.map((edge) => `${result.imports.nodes[edge.from]}>${result.imports.nodes[edge.to]}`);
  assert.ok(edges.includes("src/app.ts>src/lib/util.ts"), edges.join(" "));
  assert.ok(edges.includes("pkg/api.py>pkg/core.py"), edges.join(" "));
  assert.ok(result.functions.edges.length >= 3);
  assert.ok(result.tree.ascii.includes("└── "));
  assert.ok(Array.isArray(result.hotspots));
}

async function verifyPhasesAndAbort(dir) {
  const partial = await scanRepository({ gitDir: dir, phases: ["languages", "tree"] });
  assert.equal(partial.imports, undefined);
  assert.ok(partial.tree);
  const controller = new AbortController();
  controller.abort();
  const aborted = (error) => error instanceof CodeInsightsError && error.code === "aborted";
  await assert.rejects(scanRepository({ gitDir: dir, signal: controller.signal }), aborted);
  await assert.rejects(scanRepository({ gitDir: path.join(tempRoot, "missing") }), (error) => error.code === "not-a-repository");
}

async function verifyDetect() {
  assert.equal((await detectLanguage({ content: "fn main() {}", path: "src/main.rs" })).name, "Rust");
  assert.equal((await detectLanguage({ content: "#!/usr/bin/env python3\nprint(1)", path: "bin/tool" })).name, "Python");
  assert.equal(await detectLanguage({ content: "", path: "notes.zzz-unknown" }), null);
}

async function main() {
  const dir = await createRepository(path.join(tempRoot, "sample"), SAMPLE_FILES);
  await verifyProgressAndResult(dir);
  await verifyPhasesAndAbort(dir);
  await verifyDetect();
  log.info("verify.scan", "Scan verification succeeded.");
}

await main();
