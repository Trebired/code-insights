import type { CodeInsightsResult, CodeMapData, CodeMapEdge, CodeMapLevel, CodeMapNode } from "#dzbbbwiozeyj";

type CodeMapOptions = {
  maxNodes?: number;
};

const ROOT_GROUP = ".";

function folderAt(path: string, depth: number): string {
  const parts = path.split("/").slice(0, -1);
  return parts.length ? parts.slice(0, depth).join("/") : ROOT_GROUP;
}

function chooseDepth(paths: string[]): number {
  for (let depth = 1; depth <= 4; depth += 1) {
    const groups = new Set(paths.map((path) => folderAt(path, depth))).size;
    if (groups > 24) return Math.max(1, depth - 1);
    if (groups >= 4) return depth;
  }
  return 4;
}

function fileNodes(result: CodeInsightsResult): { edges: CodeMapEdge[]; nodes: CodeMapNode[] } {
  const graph = result.imports;
  if (!graph) return { edges: [], nodes: [] };
  const lines = new Map(result.files.map((file) => [file.path, file]));
  const nodes = graph.nodes.map((path) => ({
        group: "",
        id: path,
        kind: lines.get(path)?.language || "",
        label: path.split("/").pop() || path,
        line: null,
        path,
        size: Math.max(1, lines.get(path)?.lines.code || 1),
  }));
  const edges = graph.edges.map((edge) => ({ source: graph.nodes[edge.from], target: graph.nodes[edge.to], weight: 1 }));
  return { edges, nodes };
}

function functionNodes(result: CodeInsightsResult): { edges: CodeMapEdge[]; nodes: CodeMapNode[] } {
  const graph = result.functions;
  if (!graph) return { edges: [], nodes: [] };
  const nodes = graph.nodes.map((node, index) => ({
        group: "",
        id: `f${index}`,
        kind: node.kind,
        label: node.owner ? `${node.owner}.${node.name}` : node.name,
        line: node.line,
        path: graph.files[node.file],
        size: Math.max(1, node.end_line - node.line + 1),
  }));
  const edges = graph.edges.map((edge) => ({ source: `f${edge.from}`, target: `f${edge.to}`, weight: edge.count }));
  return { edges, nodes };
}

function keepConnected(nodes: CodeMapNode[], edges: CodeMapEdge[], maxNodes: number) {
  if (nodes.length <= maxNodes) return { edges, nodes, truncated: false };
  const degree = new Map<string, number>();
  for (const edge of edges) {
    degree.set(edge.source, (degree.get(edge.source) || 0) + edge.weight);
    degree.set(edge.target, (degree.get(edge.target) || 0) + edge.weight);
  }
  const ranked = [...nodes].sort((left, right) => (degree.get(right.id) || 0) - (degree.get(left.id) || 0));
  const kept = new Set(ranked.slice(0, maxNodes).map((node) => node.id));
  return {
    edges: edges.filter((edge) => kept.has(edge.source) && kept.has(edge.target)),
    nodes: nodes.filter((node) => kept.has(node.id)),
    truncated: true,
  };
}

function buildCodeMapData(result: CodeInsightsResult, level: CodeMapLevel, options: CodeMapOptions = {}): CodeMapData {
  const source = level === "function" ? functionNodes(result) : fileNodes(result);
  const capped = keepConnected(source.nodes, source.edges, Math.max(50, options.maxNodes || 4000));
  const depth = chooseDepth(capped.nodes.map((node) => node.path));
  const sizes = new Map<string, number>();
  for (const node of capped.nodes) {
    node.group = folderAt(node.path, depth);
    sizes.set(node.group, (sizes.get(node.group) || 0) + 1);
  }
  const groups = [...sizes.entries()]
  .map(([id, size]) => ({ id, label: id === ROOT_GROUP ? "/" : `${id}/`, size }))
  .sort((left, right) => right.size - left.size);
  const truncated = capped.truncated || Boolean(level === "function" ? result.functions?.truncated : result.imports?.truncated);
  return { edges: capped.edges, groups, level, nodes: capped.nodes, truncated };
}

export { buildCodeMapData };
export type { CodeMapOptions };
