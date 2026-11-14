import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

async function importDist(subpath) {
  return await import(path.join(rootDir, "dist", subpath));
}

async function createRepository(dir, files) {
  await fs.rm(dir, { force: true, recursive: true });
  for (const [file, content] of Object.entries(files)) {
    const target = path.join(dir, file);
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, content);
  }
  const env = { ...process.env, GIT_AUTHOR_EMAIL: "v@v", GIT_AUTHOR_NAME: "v", GIT_COMMITTER_EMAIL: "v@v", GIT_COMMITTER_NAME: "v" };
  for (const args of [["init", "-q", "-b", "main"], ["add", "-A"], ["commit", "-q", "-m", "one"]]) {
    execFileSync("git", args, { cwd: dir, env });
  }
  return dir;
}

const SAMPLE_FILES = {
  "pkg/api.py": "from .core import compute\n\ndef handler():\n    compute()\n",
  "pkg/core.py": "def compute():\n    return 1\n",
  "pkg/__init__.py": "",
  "src/app.ts": "import { helper } from './lib/util';\nexport function main() { helper(); }\n",
  "src/lib/util.ts": "export function helper() { return 1; }\n",
  "src/view.tsx": "import { main } from '../src/app';\nexport const View = () => { main(); return null; };\n",
};

export { createRepository, importDist, SAMPLE_FILES };
