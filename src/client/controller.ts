import { copyFor } from "#0no5pre0uzbk";
import type { CodeMapData, CodeMapLevel } from "#dzbbbwiozeyj";
import { layout } from "./layout.js";
import { buildView, clusterId, type ViewGraph } from "./model.js";
import { highlight, render, scaleLabels, type Rendered } from "./render.js";
import { apply, bindZoomPan, fit, type Transform } from "./viewport.js";

type MapState = {
  canvas: SVGSVGElement;
  data: CodeMapData | null;
  filter: string;
  graph: ViewGraph | null;
  level: CodeMapLevel;
  open: Set<string>;
  positions: Map<string, {x:number;y:number}>;
  rendered: Rendered | null;
  root: HTMLElement;
  selected: string | null;
  status: HTMLElement | null;
  transform: Transform;
};

function copyOf(state: MapState) {
  return copyFor(state.root.getAttribute("data-code-map-lang"));
}

function setStatus(state: MapState, text: string) {
  if (state.status) state.status.textContent = text;
}

function neighbours(state: MapState, id: string): Set<string> {
  const focus = new Set([id]);
  for (const edge of state.graph?.edges || []) {
    if (edge.source === id) focus.add(edge.target);
    if (edge.target === id) focus.add(edge.source);
  }
  return focus;
}

function filterFocus(state: MapState): Set<string>|null {
  const query = state.filter.trim().toLowerCase();
  if (!query || !state.data) return null;
  const focus = new Set<string>();
  for (const node of state.data.nodes) {
    if (!node.label.toLowerCase().includes(query) && !node.path.toLowerCase().includes(query)) continue;
    focus.add(state.open.has(node.group) ? node.id : clusterId(node.group));
  }
  return focus;
}

function refreshHighlight(state: MapState) {
  if (!state.rendered) return;
  const focus = state.selected ? neighbours(state, state.selected) : filterFocus(state);
  const members = state.graph ? state.graph.nodes.filter((node) => !node.cluster).length : 0;
  highlight(state.rendered, focus, members <= 60);
}

function draw(state: MapState, refit: boolean) {
  if (!state.data) return;
  const graph = buildView(state.data, state.open);
  const box = state.canvas.getBoundingClientRect();
  layout(graph, state.positions, box.width && box.height ? box.height / box.width : 1);
  for (const node of graph.nodes) state.positions.set(node.id, { x: node.x || 0, y: node.y || 0 });
  state.graph = graph;
  state.rendered = render(state.canvas, graph);
  if (refit) state.transform = fit(state.canvas, graph);
  apply(state.rendered.viewport, state.transform);
  scaleLabels(state.rendered, state.transform.k);
  refreshHighlight(state);
  const copy = copyOf(state);
  const stats = `${state.data.nodes.length} ${copy("nodes")}, ${state.data.edges.length} ${copy("links")}`;
  setStatus(state, state.data.nodes.length ? (state.data.truncated ? `${stats}. ${copy("truncated")}` : stats) : copy("empty"));
}

async function load(state: MapState, level: CodeMapLevel) {
  state.level = level;
  state.selected = null;
  state.open = new Set();
  setStatus(state, copyOf(state)("loading"));
  const url = String(state.root.getAttribute("data-code-map-url") || "");
  try {
    const response = await fetch(
      `${url}${url.includes("?") ? "&" : "?"}level=${level}`,
      { credentials: "same-origin", headers: { Accept: "application/json" } }
    );
    if (!response.ok) throw new Error(String(response.status));
    const body = await response.json();
    state.data = (body && body.data) || body;
    if (state.data && state.data.groups.length === 1) state.open.add(state.data.groups[0].id);
    draw(state, true);
  } catch {
    state.data = null;
    state.canvas.replaceChildren();
    setStatus(state, copyOf(state)("failed"));
  }
}

function navigate(state: MapState, id: string) {
  const template = String(state.root.getAttribute("data-code-map-href") || "");
  const node = state.graph?.nodes.find((item) => item.id === id);
  if (!template || !node) return;
  const href = template.replace("{path}", encodeURIComponent(node.path)).replace("{line}", String(node.line || 1));
  const detail = { href, line: node.line, path: node.path };
  const event = new CustomEvent("code-map:open", { bubbles: true, cancelable: true, detail });
  if (state.root.dispatchEvent(event)) window.location.assign(href);
}

function onNodeClick(state: MapState, id: string) {
  const node = state.graph?.nodes.find((item) => item.id === id);
  if (!node) return;
  if (node.cluster) {
    state.open.add(node.group);
    state.selected = null;
    draw(state, true);
    return;
  }
  if (state.selected === id) return navigate(state, id);
  state.selected = id;
  refreshHighlight(state);
}

function levelButtons(root: HTMLElement): HTMLElement[] {
  let scope: HTMLElement | null = root.parentElement;
  for (let depth = 0; scope && depth < 8; depth += 1, scope = scope.parentElement) {
    const found = Array.from(scope.querySelectorAll<HTMLElement>("[data-code-map-switch]"));
    if (found.length) return found;
  }
  return [];
}

export { draw, levelButtons, load, onNodeClick, refreshHighlight };
export type { MapState };

export function createState(root: HTMLElement, canvas: SVGSVGElement): MapState {
  return {
    canvas, data: null, filter: "", graph: null,
    level: root.getAttribute("data-code-map-level") === "function" ? "function" : "file",
    open: new Set(), positions: new Map(), rendered: null, root, selected: null,
    status: root.querySelector<HTMLElement>("[data-code-map-status]"), transform: { k: 1, x: 0, y: 0 },
  };
}

export function bindState(state: MapState) {
  bindZoomPan(state.canvas, () => state.transform, (next) => {
      state.transform = next;
      if (!state.rendered) return;
      apply(state.rendered.viewport, next);
      scaleLabels(state.rendered, next.k);
  });
}
