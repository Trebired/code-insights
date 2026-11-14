import { bindState, createState, levelButtons, load, onNodeClick, refitOnResize, refreshHighlight, type MapState } from "./controller.js";

const bound = new WeakMap<HTMLElement, MapState>();

function bindClicks(state: MapState) {
  let downAt: { x: number; y: number } | null = null;
  state.canvas.addEventListener("pointerdown", (event) => { downAt = { x: event.clientX, y: event.clientY }; });
  state.canvas.addEventListener("click", (event) => {
      const moved = downAt && Math.hypot(event.clientX - downAt.x, event.clientY - downAt.y) > 4;
      if (moved) return;
      const node = (event.target as Element).closest("[data-code-map-node]");
      if (node) return onNodeClick(state, String(node.getAttribute("data-code-map-node")));
      state.selected = null;
      refreshHighlight(state);
  });
}

function bindControls(state: MapState) {
  const filter = state.root.querySelector<HTMLInputElement>("[data-code-map-filter]");
  filter?.addEventListener("input", () => {
      state.filter = filter.value;
      state.selected = null;
      refreshHighlight(state);
  });
  const buttons = levelButtons(state.root);
  for (const button of buttons) {
    button.addEventListener("click", () => {
        const level = button.getAttribute("data-code-map-switch") === "function" ? "function" : "file";
        buttons.forEach((item) => item.setAttribute("aria-pressed", item === button ? "true" : "false"));
        void load(state, level);
    });
  }
}

function bindCodeMap(root: HTMLElement) {
  if (bound.has(root)) return;
  const canvas = root.querySelector<SVGSVGElement>("svg[data-code-map-canvas]");
  if (!canvas) return;
  const state = createState(root, canvas);
  bound.set(root, state);
  bindState(state);
  bindClicks(state);
  bindControls(state);
  refitOnResize(state);
  void load(state, state.level);
}

function bindCodeMaps(scope: ParentNode = document) {
  scope.querySelectorAll<HTMLElement>("[data-code-map-root]").forEach(bindCodeMap);
}

function reloadCodeMaps(scope: ParentNode = document) {
  scope.querySelectorAll<HTMLElement>("[data-code-map-root]").forEach((root) => {
      const state = bound.get(root);
      if (state) void load(state, state.level);
      else bindCodeMap(root);
  });
}

export { bindCodeMap, bindCodeMaps, reloadCodeMaps };
