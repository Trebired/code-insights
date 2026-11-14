import { forceCenter, forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY } from "d3-force";

import type { ViewGraph, ViewNode } from "./model.js";

function radius(node: ViewNode): number {
  return node.cluster ? 10 + Math.sqrt(node.count) * 2.4 : 4 + Math.min(10, Math.sqrt(node.size) / 2);
}

function seed(graph: ViewGraph, previous: Map<string, {x:number;y:number}>) {
  graph.nodes.forEach((node, index) => {
      const known = previous.get(node.id) || previous.get(`group:${node.group}`);
      const angle = index * 2.399963;
      const spread = known ? 18 : 40 + Math.sqrt(index) * 30;
      node.x = (known ? known.x : 0) + Math.cos(angle) * spread;
      node.y = (known ? known.y : 0) + Math.sin(angle) * spread;
  });
}

function layout(graph: ViewGraph, previous: Map<string, {x:number;y:number}>, aspect = 1): void {
  seed(graph, previous);
  const links = graph.edges.map((edge) => ({ ...edge }));
  const simulation = forceSimulation(graph.nodes as any)
  .force(
    "link",
    forceLink(links as any).id((node: any) => node.id).distance((link: any) => 56 + radius(link.source) + radius(link.target)).strength(0.2)
  )
  .force("charge", forceManyBody().strength((node: any) => (node.cluster ? -320 : -150)))
  .force("collide", forceCollide((node: any) => radius(node) + (node.cluster ? 26 : 22)).iterations(2))
  .force("x", forceX(0).strength(0.04 * Math.max(1, aspect)))
  .force("y", forceY(0).strength(0.04 / Math.max(1, 1 / aspect)))
  .force("center", forceCenter(0, 0))
  .stop();
  const ticks = graph.nodes.length > 600 ? 160 : 300;
  for (let tick = 0; tick < ticks; tick += 1) simulation.tick();
}

export { layout, radius };
