import { spawn } from "node:child_process";
import readline from "node:readline";

import { resolveBinary } from "./binary.js";

class CodeInsightsError extends Error {
  code: string;

  constructor(code: string, message: string) {
    super(message || code);
    this.code = code;
    this.name = "CodeInsightsError";
  }
}

type EngineRun = {
  args: string[];
  input?: string;
  onEvent: (event: Record<string, any>) => void;
  signal?: AbortSignal;
};

function parseLine(line: string): Record<string, any>|null {
  try {
    const value = JSON.parse(line);
    return value && typeof value === "object" ? value : null;
  } catch {
    return null;
  }
}

function runEngine(run: EngineRun): Promise<void> {
  return new Promise((resolve, reject) => {
      if (run.signal?.aborted) return reject(new CodeInsightsError("aborted", "Scan aborted."));
      const child = spawn(resolveBinary(), run.args, { stdio: ["pipe", "pipe", "pipe"] });
      let failure: CodeInsightsError | null = null;
      let stderr = "";
      const onAbort = () => {
        failure = new CodeInsightsError("aborted", "Scan aborted.");
        child.kill("SIGKILL");
      };
      run.signal?.addEventListener("abort", onAbort, { once: true });
      readline.createInterface({ input: child.stdout }).on("line", (line) => {
          const event = parseLine(line);
          if (!event) return;
          if (event.type === "error") failure = failure || new CodeInsightsError(String(event.code), String(event.message || ""));
          else run.onEvent(event);
      });
      child.stderr.on("data", (chunk) => {
          stderr = (stderr + String(chunk)).slice(-4000);
      });
      child.on("error", (error) => reject(new CodeInsightsError("engine-failed", error.message)));
      child.on("close", (code) => {
          run.signal?.removeEventListener("abort", onAbort);
          if (failure) return reject(failure);
          if (code !== 0) return reject(new CodeInsightsError("engine-failed", stderr.trim() || `engine exited with ${code}`));
          resolve();
      });
      child.stdin.end(run.input ?? "");
  });
}

export { CodeInsightsError, runEngine };
