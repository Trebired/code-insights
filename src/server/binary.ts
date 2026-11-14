import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const BINARY_ENV = "TREBIRED_CODE_INSIGHTS_BINARY";

function packageRoot(): string {
  let dir = path.dirname(fileURLToPath(import.meta.url));
  for (let depth = 0; depth < 5; depth += 1) {
    if (fs.existsSync(path.join(dir, "package.json"))) return dir;
    dir = path.dirname(dir);
  }
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
}

function libcVariant(): "gnu" | "musl" {
  const report = typeof process.report?.getReport === "function" ? (process.report.getReport() as any) : null;
  if (report && report.header && report.header.glibcVersionRuntime) return "gnu";
  return fs.existsSync("/etc/alpine-release") ? "musl" : "gnu";
}

function platformDirName(): string {
  if (process.platform !== "linux") return "";
  if (process.arch === "x64") return `linux-x64-${libcVariant()}`;
  if (process.arch === "arm64") return `linux-arm64-${libcVariant()}`;
  return "";
}

function binaryCandidates(): string[] {
  const root = packageRoot();
  const fromEnv = String(process.env[BINARY_ENV] || "").trim();
  const platform = platformDirName();
  return [
    fromEnv,
    platform ? path.join(root, "native", platform, "code-insights") : "",
    path.join(root, "native", "code-insights", "target", "release", "code-insights"),
  ].filter(Boolean);
}

function resolveBinary(): string {
  const found = binaryCandidates().find((candidate) => fs.existsSync(candidate));
  if (found) return found;
  throw new Error(`code-insights-binary-missing: no engine for ${process.platform}-${process.arch}; set ${BINARY_ENV}`);
}

export { BINARY_ENV, resolveBinary };
