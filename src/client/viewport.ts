import type { ViewGraph } from "./model.js";
import { radius } from "./layout.js";

type Transform = { k: number; x: number; y: number };

function bounds(graph: ViewGraph, labelScale: number) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const node of graph.nodes) {
    const label = node.cluster ? `${node.label} (${node.count})` : node.label;
    const halfWidth = Math.max(radius(node), label.length * (node.cluster ? 3.6 : 3) * labelScale) + 8;
    const height = radius(node) + 24 * labelScale;
    minX = Math.min(minX, (node.x || 0) - halfWidth);
    maxX = Math.max(maxX, (node.x || 0) + halfWidth);
    minY = Math.min(minY, (node.y || 0) - height);
    maxY = Math.max(maxY, (node.y || 0) + height);
  }
  return { maxX, maxY, minX, minY };
}

function fit(canvas: SVGSVGElement, graph: ViewGraph): Transform {
  const box = canvas.getBoundingClientRect();
  const width = box.width || 800;
  const height = box.height || 500;
  if (!graph.nodes.length) return { k: 1, x: width / 2, y: height / 2 };
  let scale = 1;
  let area = bounds(graph, 1);
  for (let pass = 0; pass < 2; pass += 1) {
    area = bounds(graph, 1 / Math.max(0.55, Math.min(1, scale)));
    scale = Math.min(2, Math.min(width / (area.maxX - area.minX), height / (area.maxY - area.minY)));
  }
  return { k: scale, x: width / 2 - ((area.minX + area.maxX) / 2) * scale, y: height / 2 - ((area.minY + area.maxY) / 2) * scale };
}

function apply(viewport: SVGGElement, transform: Transform) {
  viewport.setAttribute("transform", `translate(${transform.x},${transform.y}) scale(${transform.k})`);
}

function bindZoomPan(canvas: SVGSVGElement, get: () => Transform, set: (next: Transform) => void): () => void {
  let drag: { x: number; y: number; start: Transform } | null = null;
  const onWheel = (event: WheelEvent) => {
    event.preventDefault();
    const box = canvas.getBoundingClientRect();
    const current = get();
    const scale = Math.max(0.2, Math.min(6, current.k * Math.exp(-event.deltaY * 0.0015)));
    const px = event.clientX - box.left;
    const py = event.clientY - box.top;
    set({ k: scale, x: px - ((px - current.x) / current.k) * scale, y: py - ((py - current.y) / current.k) * scale });
  };
  const onDown = (event: PointerEvent) => {
    if ((event.target as Element).closest("[data-code-map-node]")) return;
    drag = { start: get(), x: event.clientX, y: event.clientY };
    canvas.setPointerCapture(event.pointerId);
  };
  const onMove = (event: PointerEvent) => {
    if (!drag) return;
    set({ ...drag.start, x: drag.start.x + event.clientX - drag.x, y: drag.start.y + event.clientY - drag.y });
  };
  const onUp = () => { drag = null; };
  canvas.addEventListener("wheel", onWheel, { passive: false });
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  return () => {
    canvas.removeEventListener("wheel", onWheel);
    canvas.removeEventListener("pointerdown", onDown);
    canvas.removeEventListener("pointermove", onMove);
    canvas.removeEventListener("pointerup", onUp);
  };
}

export { apply, bindZoomPan, fit };
export type { Transform };
