import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Window } from "happy-dom";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { resolveLogger } from "@package/logger-adapter";

import { createRepository, importDist, SAMPLE_FILES } from "./fixtures.mjs";

const { buildCodeMapData, scanRepository } = await importDist("server/index.js");

const log = resolveLogger({ source: "@trebired/code-insights" });
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

function installDom() {
  const window = new Window({ url: "https://example.test/repo" });
  const keys = ["window", "document", "Node", "Element", "HTMLElement", "SVGElement", "HTMLInputElement"];
  for (const key of [...keys, "Event", "CustomEvent", "MouseEvent", "PointerEvent"]) {
    globalThis[key] = key === "window" ? window : window[key];
  }
  return window;
}

function verifyData(result) {
  const files = buildCodeMapData(result, "file");
  assert.equal(files.level, "file");
  assert.ok(files.nodes.every((node) => node.group), "every node has a folder group");
  assert.ok(files.edges.some((edge) => edge.source === "src/app.ts" && edge.target === "src/lib/util.ts"));
  const functions = buildCodeMapData(result, "function");
  assert.ok(functions.nodes.some((node) => node.label === "helper" && node.line === 1));
  const capped = buildCodeMapData(result, "file", { maxNodes: 1 });
  assert.ok(capped.nodes.length <= 50);
  return files;
}

async function renderMarkup() {
  const { code_map, hotspot_list } = await importDist("react/index.js");
  const mapProps = { dataUrl: "/insights/graph", hrefTemplate: "/edit?path={path}&line={line}", lang: "cs" };
  const map = renderToStaticMarkup(createElement(() => code_map(mapProps)));
  assert.ok(map.includes("data-code-map-root") && map.includes("Soubory") && map.includes("Funkce"), "map shell renders with copy");
  const sample = [{ commits: 4, language: "TypeScript", lines: 90, path: "src/app.ts", score: 100 }];
  const hotspots = renderToStaticMarkup(createElement(() => hotspot_list({ hotspots: sample })));
  assert.ok(hotspots.includes("src/app.ts") && hotspots.includes("4 commits") && !hotspots.includes("1 commits") && !hotspots.includes("1 commits"));
  return map;
}

async function settle() {
  for (let index = 0; index < 5; index += 1) await new Promise((resolve) => setTimeout(resolve, 10));
}

async function verifyClient(markup, data) {
  const window = installDom();
  document.body.innerHTML = markup;
  const levelOf = (url) => (String(url).includes("function") ? "function" : "file");
  globalThis.fetch = async(url) => ({ json: async() => ({ data: { ...data, level: levelOf(url) } }), ok: true });
  const assigned = [];
  document.addEventListener("code-map:open", (event) => {
      event.preventDefault();
      assigned.push(event.detail.href);
  });
  const { bindCodeMaps } = await importDist("client/index.js");
  bindCodeMaps(document);
  await settle();
  const nodes = () => Array.from(document.querySelectorAll("[data-code-map-node]"));
  assert.equal(nodes().length, data.groups.length, "folders start collapsed");
  const cluster = nodes().find((node) => node.getAttribute("data-code-map-node") === "group:src");
  cluster.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
  const app = nodes().find((node) => node.getAttribute("data-code-map-node") === "src/app.ts");
  assert.ok(app, "opening a folder shows its files");
  app.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
  assert.ok(nodes().some((node) => node.getAttribute("opacity") === "0.15"), "selecting a file dims unrelated nodes");
  app.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
  assert.deepEqual(assigned, ["/edit?path=src%2Fapp.ts&line=1"], "a second click opens the file");
  const fetched = [];
  const plain = globalThis.fetch;
  globalThis.fetch = async(url) => { fetched.push(String(url)); return plain(url); };
  document.querySelector("[data-code-map-switch='function']").dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
  await settle();
  assert.deepEqual(fetched, ["/insights/graph?level=function"], "the level switch reloads at function level");
  const { reloadCodeMaps } = await importDist("client/index.js");
  reloadCodeMaps(document);
  await settle();
  assert.deepEqual(fetched.at(-1), "/insights/graph?level=function", "a reload keeps the current level");
  await verifyHiddenMap(markup, data);
  await window.happyDOM.close();
}

async function verifyHiddenMap(markup, data) {
  const observers = [];
  globalThis.ResizeObserver = class {
    constructor(callback) { this.callback = callback; observers.push(this); }
    observe() {}
  };
  document.body.innerHTML = markup;
  globalThis.fetch = async() => ({ json: async() => ({ data }), ok: true });
  const canvas = document.querySelector("svg[data-code-map-canvas]");
  let box = { height: 0, left: 0, top: 0, width: 0 };
  canvas.getBoundingClientRect = () => box;
  const { bindCodeMaps } = await importDist("client/index.js");
  bindCodeMaps(document);
  await settle();
  const transform = () => canvas.querySelector("g").getAttribute("transform");
  const hidden = transform();
  box = { height: 420, left: 0, top: 0, width: 360 };
  for (const observer of observers) observer.callback([]);
  await settle();
  assert.notEqual(transform(), hidden, "a map bound inside a hidden tab refits once it is shown");
  delete globalThis.ResizeObserver;
}

async function main() {
  const dir = await createRepository(path.join(rootDir, ".tmp", "verify-map", "sample"), SAMPLE_FILES);
  const result = await scanRepository({ gitDir: dir });
  const data = verifyData(result);
  const markup = await renderMarkup();
  await verifyClient(markup, data);
  log.info("verify.map", "Code map verification succeeded.");
}

await main();
