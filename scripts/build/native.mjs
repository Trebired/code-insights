import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolveLogger } from "@package/logger-adapter";

const log = resolveLogger({ source: "@trebired/code-insights" });
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const manifest = path.join(rootDir, "native", "code-insights", "Cargo.toml");
const TARGET_DIRS = new Map([
    ["x86_64-unknown-linux-gnu", "linux-x64-gnu"],
    ["aarch64-unknown-linux-gnu", "linux-arm64-gnu"],
    ["x86_64-unknown-linux-musl", "linux-x64-musl"],
    ["aarch64-unknown-linux-musl", "linux-arm64-musl"],
]);

function argValue(flag) {
  const index = process.argv.indexOf(flag);
  return index === -1 ? "" : String(process.argv[index + 1] || "");
}

function hostTarget() {
  const result = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
  const line = String(result.stdout || "").split("\n").find((item) => item.startsWith("host:"));
  return line ? line.slice(5).trim() : "";
}

function buildNative() {
  const target = argValue("--target") || process.env.TREBIRED_CODE_INSIGHTS_TARGET || hostTarget();
  const dirName = TARGET_DIRS.get(target);
  if (!dirName) throw new Error(`unsupported-native-target: ${target}`);
  const result = spawnSync("cargo", ["build", "--release", "--target", target, "--manifest-path", manifest], { cwd: rootDir, stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status || 1);
  const built = path.join(rootDir, "native", "code-insights", "target", target, "release", "code-insights");
  const destination = path.join(rootDir, "native", dirName, "code-insights");
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.copyFileSync(built, destination);
  fs.chmodSync(destination, 0o755);
  log.info("build.native", `engine ready at ${path.relative(rootDir, destination)}`);
}

buildNative();
