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
  assert.deepEqual(started, ["listing", "license", "languages", "tree", "imports", "functions", "hotspots"]);
  assert.deepEqual(result.license.licenses, []);
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

const MIT_TEXT = `MIT License

Copyright (c) 2026 Example Ltd

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
`;

async function verifyLicense() {
  const dir = await createRepository(path.join(tempRoot, "licensed"), { "LICENSE.md": MIT_TEXT, "index.js": "module.exports = 1;\n" });
  const result = await scanRepository({ gitDir: dir, phases: ["license"] });
  assert.equal(result.license.licenses.length, 1);
  const [match] = result.license.licenses;
  assert.deepEqual(
    { id: match.id, osi: match.osi_approved, path: match.path, source: match.source },
    { id: "MIT", osi: true, path: "LICENSE.md", source: "file" }
  );
  assert.deepEqual(result.license.files, ["LICENSE.md"]);
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
  await verifyLicense();
  log.info("verify.scan", "Scan verification succeeded.");
}

await main();
