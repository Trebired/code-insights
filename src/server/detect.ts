import { runEngine } from "./process.js";

type DetectedLanguage = {
  color: string;
  id: string;
  name: string;
  type: string;
};

async function detectLanguage(input: { content?: string; path: string }): Promise<DetectedLanguage|null> {
  let language: DetectedLanguage | null = null;
  await runEngine({
      args: ["detect", "--path", String(input.path || "")],
      input: String(input.content || ""),
      onEvent: (event) => {
        if (event.type === "language") language = event.language || null;
      },
  });
  return language;
}

export { detectLanguage };
export type { DetectedLanguage };
