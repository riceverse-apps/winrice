import { createI18n } from "vue-i18n";
import en from "@locales/en.json";
import ptBR from "@locales/pt-BR.json";

export const LOCALES = [
  { code: "en", label: "English" },
  { code: "pt-BR", label: "Português (Brasil)" },
] as const;

function detect(): string {
  try {
    const saved = localStorage.getItem("winrice.locale");
    if (saved) return saved;
  } catch {
    /* ignore */
  }
  return navigator.language.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const i18n = createI18n({
  legacy: false,
  locale: detect(),
  fallbackLocale: "en",
  messages: { en, "pt-BR": ptBR },
});

export function setLocale(code: string): void {
  i18n.global.locale.value = code as "en" | "pt-BR";
  document.documentElement.lang = code;
  try {
    localStorage.setItem("winrice.locale", code);
  } catch {
    /* ignore */
  }
}
