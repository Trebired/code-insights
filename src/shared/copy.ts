const COPY = {
  cs: {
    codeMap: "Mapa kódu",
    empty: "Žádná propojení.",
    failed: "Mapu kódu se nepodařilo načíst.",
    files: "Soubory",
    filter: "Filtrovat podle názvu",
    functions: "Funkce",
    hint: "Kliknutím na složku ji otevřete. Kliknutím na soubor zobrazíte jeho vazby, dalším kliknutím ho otevřete.",
    hotspots: "Rizikové soubory",
    hotspotsDescription: "Velké soubory, které se často mění.",
    hotspotsEmpty: "Zatím žádné rizikové soubory.",
    links: "vazeb",
    loading: "Načítání mapy kódu…",
    nodes: "uzlů",
    truncated: "Zobrazena je nejpropojenější část velkého grafu.",
  },
  en: {
    codeMap: "Code map",
    empty: "No connections found.",
    failed: "The code map could not be loaded.",
    files: "Files",
    filter: "Filter by name",
    functions: "Functions",
    hint: "Click a folder to open it. Click a file to see its links, then click it again to open it.",
    hotspots: "Hotspots",
    hotspotsDescription: "Large files that change often.",
    hotspotsEmpty: "No hotspots yet.",
    links: "links",
    loading: "Loading the code map…",
    nodes: "nodes",
    truncated: "Showing the most connected part of a large graph.",
  },
} as const;

type CopyKey = keyof typeof COPY.en;

const COUNTED = {
  cs: {
    commits: { few: "commity", many: "commitů", one: "commit", other: "commitů" },
    lines: { few: "řádky", many: "řádků", one: "řádek", other: "řádků" },
  },
  en: { commits: { one: "commit", other: "commits" }, lines: { one: "line", other: "lines" } },
} as const;

function countFor(lang?: string | null) {
  const base = String(lang || "").toLowerCase().split("-")[0] === "cs" ? "cs" : "en";
  const rules = new Intl.PluralRules(base);
  return (count: number, noun: "commits" | "lines") => {
    const forms: Record<string, string> = COUNTED[base][noun];
    return `${count} ${forms[rules.select(count)] || forms.other}`;
  };
}

function copyFor(lang?: string | null) {
  const base = String(lang || "").toLowerCase().split("-")[0];
  const table = base === "cs" ? COPY.cs : COPY.en;
  return (key: CopyKey) => table[key];
}

export { copyFor, countFor };
export type { CopyKey };
