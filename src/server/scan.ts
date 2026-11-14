import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import type { LoggerAdapterLogger } from "@package/logger-adapter";

import { RESULT_VERSION, SCAN_PHASES } from "#5r46asnbaeq8";
import type { CodeInsightsResult, ScanPhase, ScanProgress } from "#dzbbbwiozeyj";
import { insightsLog } from "./logging.js";
import { CodeInsightsError, runEngine } from "./process.js";

type ScanLimits = {
  historyCommits?: number;
  maxFunctionEdges?: number;
  maxFunctionNodes?: number;
  maxImportEdges?: number;
  maxParseBytes?: number;
  maxReadBytes?: number;
};

type ScanPhaseEvent = {
  ms?: number;
  phase: ScanPhase;
  state: "done" | "start";
  total?: number;
};

type ScanRepositoryOptions = {
  gitDir: string;
  limits?: ScanLimits;
  logger?: LoggerAdapterLogger | null;
  onPhase?: (event: ScanPhaseEvent) => void;
  onProgress?: (progress: ScanProgress) => void;
  phases?: ScanPhase[];
  ref?: string;
  signal?: AbortSignal;
};

const LIMIT_FLAGS: [keyof ScanLimits, string][] = [
  ["historyCommits", "--history"],
  ["maxFunctionEdges", "--max-function-edges"],
  ["maxFunctionNodes", "--max-function-nodes"],
  ["maxImportEdges", "--max-import-edges"],
  ["maxParseBytes", "--max-parse-bytes"],
  ["maxReadBytes", "--max-read-bytes"],
];

function scanArgs(options: ScanRepositoryOptions, out: string): string[] {
  const args = ["scan", "--git-dir", options.gitDir, "--out", out, "--ref", options.ref || "HEAD"];
  const phases = (options.phases || []).filter((phase) => SCAN_PHASES.includes(phase));
  if (phases.length) args.push("--phases", phases.join(","));
  for (const [key, flag] of LIMIT_FLAGS) {
    const value = Number(options.limits?.[key]);
    if (Number.isFinite(value) && value > 0) args.push(flag, String(Math.floor(value)));
  }
  return args;
}

function dispatchEvent(event: Record<string, any>, options: ScanRepositoryOptions) {
  if (event.type === "progress") {
    options.onProgress?.({ done: Number(event.done) || 0, percent: Number(event.percent) || 0, phase: event.phase, total: Number(event.total) || 0 });
  } else if (event.type === "phase") {
    options.onPhase?.({ ms: event.ms, phase: event.phase, state: event.state, total: event.total });
  } else if (event.type === "done") {
    options.onProgress?.({ done: 1, percent: 100, phase: "hotspots", total: 1 });
  }
}

async function readResult(out: string): Promise<CodeInsightsResult> {
  const result = JSON.parse(await fs.readFile(out, "utf8")) as CodeInsightsResult;
  if (!result || result.version !== RESULT_VERSION) {
    throw new CodeInsightsError("result-version-mismatch", `Expected result version ${RESULT_VERSION}, got ${result && result.version}.`);
  }
  return result;
}

async function scanRepository(options: ScanRepositoryOptions): Promise<CodeInsightsResult> {
  if (!options || !String(options.gitDir || "").trim()) throw new CodeInsightsError("missing-git-dir", "scanRepository() requires gitDir.");
  const log = insightsLog(options.logger);
  const workDir = await fs.mkdtemp(path.join(os.tmpdir(), "code-insights-"));
  const out = path.join(workDir, "result.json");
  const startedAt = performance.now();
  try {
    await runEngine({ args: scanArgs(options, out), onEvent: (event) => dispatchEvent(event, options), signal: options.signal });
    const result = await readResult(out);
    log.info("scan", "repository scanned", { commit: result.commit, files: result.totals.count, ms: Math.round(performance.now() - startedAt) });
    return result;
  } finally {
    await fs.rm(workDir, { force: true, recursive: true });
  }
}

export { scanRepository };
export type { ScanLimits, ScanPhaseEvent, ScanRepositoryOptions };
