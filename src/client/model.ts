import type { CodeMapData, CodeMapNode } from "#dzbbbwiozeyj";

type ViewNode = {
  cluster: boolean;
  count: number;
  group: string;
  id: string;
  label: string;
  line: number | null;
  path: string;
  size: number;
  x?: number;
  y?: number;
};

type ViewEdge = {
  source: string;
  target: string;
  weight: number;
};

type ViewGraph = {
  edges: ViewEdge[];
  nodes: ViewNode[];
};

const MAX_OPEN_GROUP_NODES = 400;

function clusterId(group: string): string {
  return `group:${group}`;
}

function clusterNode(group: { id: string; label: string; size: number }): ViewNode {
  const id = clusterId(group.id);
  return { cluster: true, count: group.size, group: group.id, id, label: group.label, line: null, path: group.id, size: group.size };
}

function memberNode(node: CodeMapNode): ViewNode {
  return { cluster: false, count: 1, group: node.group, id: node.id, label: node.label, line: node.line, path: node.path, size: node.size };
}

function degreeOf(data: CodeMapData): Map<string, number> {
  const degree = new Map<string, number>();
  for (const edge of data.edges) {
    degree.set(edge.source, (degree.get(edge.source) || 0) + 1);
    degree.set(edge.target, (degree.get(edge.target) || 0) + 1);
  }
  return degree;
}

function openMembers(data: CodeMapData, group: string, degree: Map<string, number>): CodeMapNode[] {
  const members = data.nodes.filter((node) => node.group === group);
  if (members.length <= MAX_OPEN_GROUP_NODES) return members;
  return members.sort((left, right) => (degree.get(right.id) || 0) - (degree.get(left.id) || 0)).slice(0, MAX_OPEN_GROUP_NODES);
}

function buildView(data: CodeMapData, open: Set<string>): ViewGraph {
  const degree = degreeOf(data);
  const visible = new Map<string, string>();
  const nodes: ViewNode[] = [];
  for (const group of data.groups) {
    if (!open.has(group.id)) {
      nodes.push(clusterNode(group));
      continue;
    }
    for (const member of openMembers(data, group.id, degree)) {
      nodes.push(memberNode(member));
      visible.set(member.id, member.id);
    }
  }
  const owner = new Map(data.nodes.map((node) => [node.id, visible.get(node.id) || (open.has(node.group) ? "" : clusterId(node.group))]));
  const weights = new Map<string, number>();
  for (const edge of data.edges) {
    const source = owner.get(edge.source);
    const target = owner.get(edge.target);
    if (!source || !target || source === target) continue;
    const key = `${source}\u0000${target}`;
    weights.set(key, (weights.get(key) || 0) + edge.weight);
  }
  const edges = [...weights.entries()].map(([key, weight]) => {
      const [source, target] = key.split("\u0000");
      return { source, target, weight };
  });
  return { edges, nodes };
}

export { buildView, clusterId };
export type { ViewEdge, ViewGraph, ViewNode };
