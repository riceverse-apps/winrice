import { reactive, ref, watch } from "vue";

export type Mode = "system" | "light" | "dark";
export type Density = "comfortable" | "compact";

interface Appearance {
  mode: Mode;
  accent: string | null; // null = default accent from tokens
  /** Use the accent color of the theme applied from the library. */
  accentFromTheme: boolean;
  density: Density;
  fontScale: number;
  reduceMotion: boolean;
}

const KEY = "winrice.appearance";
const defaults: Appearance = { mode: "system", accent: null, accentFromTheme: false, density: "comfortable", fontScale: 1, reduceMotion: false };

function load(): Appearance {
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem(KEY) || "{}") };
  } catch {
    return { ...defaults };
  }
}

export const appearance = reactive<Appearance>(load());

/** Accent of the applied theme (null when none). */
export const themeAccent = ref<string | null>(null);

const media = matchMedia("(prefers-color-scheme: dark)");

function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16);
  const ch = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * ch[0] + 0.7152 * ch[1] + 0.0722 * ch[2];
}

/** Picks black or white text, whichever has the higher contrast on the accent (WCAG AA). */
export function textOn(hex: string): string {
  const l = luminance(hex);
  return 1.05 / (l + 0.05) >= (l + 0.05) / 0.05 ? "#000000" : "#ffffff";
}

export function apply(): void {
  const root = document.documentElement;
  const dark = appearance.mode === "dark" || (appearance.mode === "system" && media.matches);
  root.dataset.theme = dark ? "dark" : "light";
  root.dataset.density = appearance.density;
  root.dataset.reduceMotion = String(appearance.reduceMotion || matchMedia("(prefers-reduced-motion: reduce)").matches);
  root.style.setProperty("--font-scale", String(appearance.fontScale));
  const accent = appearance.accentFromTheme && themeAccent.value ? themeAccent.value : appearance.accent;
  if (accent) {
    root.style.setProperty("--accent", accent);
    root.style.setProperty("--accent-text", textOn(accent));
  } else {
    root.style.removeProperty("--accent");
    root.style.removeProperty("--accent-text");
  }
}

export function toggleMode(): void {
  const dark = document.documentElement.dataset.theme === "dark";
  appearance.mode = dark ? "light" : "dark";
}

export function initAppearance(): void {
  apply();
  watch(appearance, () => {
    try {
      localStorage.setItem(KEY, JSON.stringify(appearance));
    } catch {
      /* storage unavailable */
    }
    apply();
  });
  watch(themeAccent, apply);
  window.addEventListener("storage", (e) => {
    if (e.key === KEY) Object.assign(appearance, load());
  });
  media.addEventListener("change", apply); // "System" follows the OS live
  window.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "l") {
      e.preventDefault();
      toggleMode();
    }
  });
}
