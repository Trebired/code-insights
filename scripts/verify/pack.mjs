import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolveLogger } from "@package/logger-adapter";

const log = resolveLogger({ source: "@trebired/code-insights" });
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const tempRoot = path.join(rootDir, ".tmp", "verify-pack");

async function main() {
  await resetTempRoot();
  const tarballPath = packPackage();
  try {
    const packageJson = readPackedPackageJson(tarballPath);
    const tarballEntries = listTarEntries(tarballPath);
    validatePackedEntrypoints(packageJson, tarballEntries);
    await validatePackedBins(packageJson, tarballPath, tarballEntries);
    validatePackedImports(packageJson, tarballEntries);
    await validatePackedEngines(tarballPath, tarballEntries);
  }
  finally {
    await fs.rm(tarballPath, { force: true });
  }
  log.info("verify.pack", "Pack verification succeeded.");
}

async function resetTempRoot() {
  await fs.rm(tempRoot, { force: true, recursive: true });
  await fs.mkdir(tempRoot, { recursive: true });
}

function packPackage() {
  const stdout = execFileSync("bun", ["pm", "pack", "--quiet", "--destination", tempRoot], {
      cwd: rootDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "inherit"],
  }).trim();
  const filename = stdout.split("\n").map((line) => line.trim()).filter(Boolean).pop();
  if (!filename) throw new Error("bun pm pack did not return a tarball filename.");
  return path.isAbsolute(filename) ? filename : path.join(tempRoot, filename);
}

function validatePackedEntrypoints(packageJson, tarballEntries) {
  const targets = new Set([packageJson.main, packageJson.types]);
  for (const value of Object.values(packageJson.exports || {})) collectExportTargets(value, targets);
  for (const target of targets) {
    if (typeof target !== "string") continue;
    assertTarEntryExists(tarballEntries, target, `Missing packed entrypoint target: ${target}`);
    assert.equal(target.includes("./src/"), false, `Packed entrypoint targets source: ${target}`);
  }
}

async function validatePackedBins(packageJson, tarballPath, tarballEntries) {
  for (const target of Object.values(packageJson.bin || {})) {
    if (typeof target !== "string") continue;
    assertTarEntryExists(tarballEntries, target, `Missing packed bin target: ${target}`);
    const localPath = path.join(rootDir, target.replace(/^\.\//u, ""));
    const localStats = await fs.stat(localPath);
    assert.notEqual(localStats.mode&0o111, 0, `Built bin is not executable: ${target}`);
    assertTarEntryExecutable(tarballPath, target);
  }
}

async function validatePackedEngines(tarballPath, tarballEntries) {
  const nativeDir = path.join(rootDir, "native");
  const engines = (await fs.readdir(nativeDir)).filter((name) => name.startsWith("linux-")).map((name) => `native/${name}/code-insights`);
  assert.ok(engines.length > 0, "Build at least one engine with bun run build:native before packing.");
  for (const engine of engines) {
    assertTarEntryExists(tarballEntries, engine, `Missing packed engine: ${engine}`);
    assertTarEntryExecutable(tarballPath, engine);
  }
  assertTarEntryExists(tarballEntries, "native/vendor/hyperpolyglot/LICENSE-MIT", "Missing the vendored hyperpolyglot licence.");
}

function validatePackedImports(packageJson, tarballEntries) {
  for (const [alias, target] of Object.entries(packageJson.imports || {})) {
    if (typeof target !== "string") continue;
    assert.equal(target.includes("./src/"), false, `Packed imports entry ${alias} still points at ${target}.`);
    assertTarEntryExists(tarballEntries, target, `Packed imports target is missing for ${alias}: ${target}`);
  }
}

function collectExportTargets(value, targets) {
  if (typeof value === "string") {
    targets.add(value);
    return;
  }
  for (const nested of Object.values(value || {})) collectExportTargets(nested, targets);
}

function assertTarEntryExists(tarballEntries, packagePath, message) {
  const entryPath = `package/${String(packagePath).replace(/^\.\//u, "")}`;
  assert.equal(tarballEntries.has(entryPath), true, message);
}

function assertTarEntryExecutable(tarballPath, packagePath) {
  const entryPath = `package/${String(packagePath).replace(/^\.\//u, "")}`;
  const listing = execFileSync("tar", ["-tvf", tarballPath, entryPath], { encoding: "utf8" });
  const mode = listing.trim().split(/\s+/u)[0] || "";
  assert.equal(mode.includes("x"), true, `Packed bin is not executable: ${packagePath}`);
}

function listTarEntries(tarballPath) {
  return new Set(execFileSync("tar", ["-tf", tarballPath], {
        encoding: "utf8",
    }).split("\n").map((entry) => entry.trim()).filter(Boolean));
}

function readPackedPackageJson(tarballPath) {
  return JSON.parse(execFileSync("tar", ["-xOf", tarballPath, "package/package.json"], {
        encoding: "utf8",
  }));
}

await main();
