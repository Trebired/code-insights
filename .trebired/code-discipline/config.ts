import { defineConfig } from "@trebired/code-discipline";

export default defineConfig({
    ignore: {
      entries: [
        { type: "folder", pattern: "native/vendor" },
        { type: "folder", pattern: "native/code-insights/target" },
      ],
      use_gitignore: true,
    },
    presets: {
      use: ["@trebired/configs/code-discipline/preset_01"],
    },
});
