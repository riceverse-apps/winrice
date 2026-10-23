import { ref } from "vue";
import { api, pick, type GlossaryTerm } from "@/api";

export const terms = ref<GlossaryTerm[]>([]);
let loading: Promise<void> | null = null;

export function loadGlossary(): Promise<void> {
  loading ??= api.getGlossary().then((g) => {
    terms.value = g;
  });
  return loading;
}

export interface Matcher {
  /** Words that trigger the tooltip, longest first. */
  words: string[];
  definition: string;
}

/** What to look for in the text of a language, and what to say about it. */
export function matchers(lang: string): Matcher[] {
  return terms.value.map((g) => {
    const term = pick(g.term, lang).text;
    const inner = /\(([^)]+)\)/.exec(term)?.[1];
    const bare = term.replace(/\s*\([^)]*\)/, "");
    const words = [bare, inner, g.term.en].filter((w): w is string => !!w && w.length > 2);
    return { words: [...new Set(words.map((w) => w.toLowerCase()))].sort((a, b) => b.length - a.length), definition: pick(g.definition, lang).text };
  });
}
