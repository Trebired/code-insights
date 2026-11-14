import { radius } from "./layout.js";
import type { ViewGraph, ViewNode } from "./model.js";

const SVG_NS = "http://www.w3.org/2000/svg";

type Rendered = {
  edges: { element: SVGLineElement; source: string; target: string }[];
  nodes: Map<string, {element:SVGGElement;label:SVGTextElement;node:ViewNode}>;
  viewport: SVGGElement;
};

function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string|number>): SVGElementTagNameMap[K] {
  const element = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attrs)) element.setAttribute(name, String(value));
  return element;
}

function hue(group: string): number {
  let hash = 0;
  for (let index = 0; index < group.length; index += 1) hash = (hash * 31 + group.charCodeAt(index)) >>> 0;
  return hash % 360;
}

function nodeColor(node: ViewNode): string {
  return `hsl(${hue(node.group)} 32% ${node.cluster ? 46 : 56}%)`;
}

function edgeElements(graph: ViewGraph, byId: Map<string, ViewNode>, layer: SVGGElement): Rendered["edges"] {
  const out: Rendered["edges"] = [];
  for (const edge of graph.edges) {
    const source = byId.get(edge.source);
    const target = byId.get(edge.target);
    if (!source || !target) continue;
    const element = svg("line", {
        "stroke-opacity": 0.22,
        "stroke-width": Math.min(4, 0.8 + Math.log2(1 + edge.weight) * 0.6),
        stroke: "currentColor",
        x1: source.x, x2: target.x, y1: source.y, y2: target.y,
    });
    layer.appendChild(element);
    out.push({ element, source: edge.source, target: edge.target });
  }
  return out;
}

function nodeElement(node: ViewNode, showLabel: boolean) {
  const group = svg("g", { "data-code-map-node": node.id, style: "cursor:pointer", transform: `translate(${node.x},${node.y})` });
  const size = radius(node);
  group.appendChild(svg("circle", { fill: nodeColor(node), r: size, stroke: "var(--background-page, transparent)", "stroke-width": 1.5 }));
  const label = svg("text", {
      "font-size": node.cluster ? 12 : 10,
      fill: "currentColor",
      "fill-opacity": node.cluster ? 0.9 : 0.75,
      "paint-order": "stroke",
      stroke: "var(--background-page, transparent)",
      "stroke-width": 3,
      "text-anchor": "middle",
      visibility: showLabel ? "visible" : "hidden",
      y: size + 12,
  });
  label.textContent = node.cluster ? `${node.label} (${node.count})` : node.label;
  const title = svg("title", {});
  title.textContent = node.cluster ? node.label : `${node.path}${node.line ? `:${node.line}` : ""}`;
  group.append(label, title);
  return { group, label };
}

function render(canvas: SVGSVGElement, graph: ViewGraph): Rendered {
  canvas.replaceChildren();
  canvas.style.color = "var(--text-color, currentColor)";
  const viewport = svg("g", { "data-code-map-viewport": "" });
  const edgeLayer = svg("g", {});
  const nodeLayer = svg("g", {});
  viewport.append(edgeLayer, nodeLayer);
  canvas.appendChild(viewport);
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const edges = edgeElements(graph, byId, edgeLayer);
  const nodes: Rendered["nodes"] = new Map();
  const labelAll = graph.nodes.filter((node) => !node.cluster).length <= 60;
  for (const node of graph.nodes) {
    const { group, label } = nodeElement(node, node.cluster || labelAll);
    nodeLayer.appendChild(group);
    nodes.set(node.id, { element: group, label, node });
  }
  return { edges, nodes, viewport };
}

function highlight(rendered: Rendered, focus: Set<string>|null, labelAll: boolean) {
  for (const [id, entry] of rendered.nodes) {
    const active = !focus || focus.has(id);
    entry.element.setAttribute("opacity", active ? "1" : "0.15");
    entry.label.setAttribute("visibility", entry.node.cluster || labelAll || (focus && focus.has(id)) ? "visible" : "hidden");
  }
  for (const edge of rendered.edges) {
    const active = !focus || (focus.has(edge.source) && focus.has(edge.target));
    edge.element.setAttribute("stroke-opacity", active ? (focus ? "0.7" : "0.22") : "0.04");
  }
}

function scaleLabels(rendered: Rendered, zoom: number) {
  const factor = 1 / Math.max(0.55, Math.min(1, zoom));
  for (const entry of rendered.nodes.values()) {
    entry.label.setAttribute("font-size", String((entry.node.cluster ? 12 : 10) * factor));
    entry.label.setAttribute("stroke-width", String(3 * factor));
  }
}

export { highlight, render, scaleLabels };
export type { Rendered };
