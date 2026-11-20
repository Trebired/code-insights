import type { SCAN_PHASES } from "./constants.js";

type ScanPhase = typeof SCAN_PHASES[number];

type CodeLines = {
  code: number;
  content: number;
  total: number;
};

type CodeTotals = {
  bytes: number;
  count: number;
  lines: CodeLines;
};

type LanguageTotal = CodeTotals& {
  color: string;
  id: string;
  name: string;
  percent: number;
  type: "data" | "markup" | "programming" | "prose";
};

type FileRecord = {
  bytes: number;
  language: string | null;
  lines: CodeLines;
  path: string;
};

type TreeNode = {
  children?: TreeNode[];
  kind: "dir" | "file";
  language: string | null;
  mode: string;
  name: string;
  object: string;
  path: string;
  size: number | null;
};

type ImportGraph = {
  edges: { from: number; kind: string; to: number }[];
  external: number;
  nodes: string[];
  truncated: boolean;
  unresolved: number;
  unresolved_samples: { from: number; spec: string }[];
};

type FunctionNode = {
  end_line: number;
  exported: boolean;
  file: number;
  kind: "arrow" | "function" | "method";
  line: number;
  name: string;
  owner: string | null;
};

type FunctionGraph = {
  edges: { confidence: "import" | "local" | "member" | "package"; count: number; from: number; to: number }[];
  files: string[];
  nodes: FunctionNode[];
  truncated: boolean;
  unresolved: number;
};

type Hotspot = {
  commits: number;
  language: string | null;
  lines: number;
  path: string;
  score: number;
};

type LicenseMatch = {
  confidence: number;
  id: string;
  name: string;
  osi_approved: boolean;
  path: string | null;
  source: "file" | "identifier" | "manifest" | "notice";
  spdx_url: string | null;
  url: string | null;
};

type LicenseAgreement = "match" | "mismatch" | "no-file" | "none" | "partial" | "undeclared" | "unverified";

type LicenseInfo = {
  agreement: LicenseAgreement;
  declared: string | null;
  declared_path: string | null;
  files: string[];
  licenses: LicenseMatch[];
};

type CodeInsightsResult = {
  commit: string;
  empty: boolean;
  files: FileRecord[];
  functions?: FunctionGraph;
  hotspots?: Hotspot[];
  imports?: ImportGraph;
  languages: LanguageTotal[];
  license?: LicenseInfo;
  ref: string;
  scanned_at: string;
  skipped: { binary: number; documentation: number; generated: number; unparsed: number; vendored: number };
  timings_ms: Partial<Record<ScanPhase, number>>;
  totals: CodeTotals;
  tree?: { ascii: string; nested: TreeNode[] };
  unknown: CodeTotals;
  version: number;
};

type ScanProgress = {
  done: number;
  percent: number;
  phase: ScanPhase;
  total: number;
};

type CodeMapLevel = "file" | "function";

type CodeMapNode = {
  group: string;
  id: string;
  kind: string;
  label: string;
  line: number | null;
  path: string;
  size: number;
};

type CodeMapEdge = {
  source: string;
  target: string;
  weight: number;
};

type CodeMapData = {
  edges: CodeMapEdge[];
  groups: { id: string; label: string; size: number }[];
  level: CodeMapLevel;
  nodes: CodeMapNode[];
  truncated: boolean;
};

export type {
  CodeInsightsResult,
  CodeLines,
  CodeMapData,
  CodeMapEdge,
  CodeMapLevel,
  CodeMapNode,
  CodeTotals,
  FileRecord,
  FunctionGraph,
  FunctionNode,
  Hotspot,
  ImportGraph,
  LanguageTotal,
  LicenseAgreement,
  LicenseInfo,
  LicenseMatch,
  ScanPhase,
  ScanProgress,
  TreeNode,
};
