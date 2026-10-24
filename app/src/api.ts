import { invoke } from "@tauri-apps/api/core";

export type I18nText = Record<string, string>;

export interface Manifest {
  id: string;
  name: string;
  category: string;
  summary: I18nText;
  difficulty: "beginner" | "intermediate" | "advanced";
  homepage: string;
  license: string;
  requires: { windows_min_build: number; windows_max_build: number | null; admin: boolean };
  conflicts: string[];
  pairs_well_with: string[];
  changes_on_screen: I18nText[];
  shortcuts: ({ keys: string } & I18nText)[];
  risks?: I18nText;
  how_to_undo?: I18nText;
  activate?: { autostart_task?: string };
  widget?: { show_by_default: boolean; quick_actions: { id: string; label: I18nText; command: string }[] };
  settings: Setting[];
  paid: boolean;
  restricted_license: boolean;
}

export interface Setting {
  key: string;
  type: "number" | "boolean" | "string" | "color";
  label: I18nText;
  default: unknown;
  suffix?: string;
  min?: number;
  max?: number;
  palette?: string;
}

export interface ToolView {
  manifest: Manifest;
  installed: boolean;
  active: boolean;
  toggleable: boolean;
  compatible: boolean;
  installed_by_app: boolean;
  favorite: boolean;
}

export type Step = { kind: string; tool?: string; [k: string]: unknown };
export interface Plan { tool: string; action: string; steps: Step[] }
export interface Outcome {
  ok: boolean;
  applied: number;
  error: string | null;
  rolled_back: boolean;
  rollback_errors: string[];
}
export interface HistoryEntry { id: number; ts: number; tool: string; action: string; status: string; detail: string | null; undoable: boolean; undone: boolean }

export interface ActionResult { outcome: Outcome; token: number | null; try_seconds: number | null }
export interface SystemInfo { build: number; edition: string; build_changed: boolean; previous_build: number | null }
export interface Reverted { tool: string; action: string; reason: "timeout" | "shortcut" | "watchdog" | "user" }

export interface Palette { name: string; colors: Record<string, string> }
export interface ThemeComponent { tool: string; config?: string; font?: string; required: boolean; mods: { id: string }[]; settings: Record<string, unknown> }
export interface Theme {
  id: string;
  name: string;
  author: { name: string; github?: string };
  version: string;
  license: string;
  description: I18nText;
  tags: string[];
  difficulty?: "beginner" | "intermediate" | "advanced";
  requires?: { windows_min_build?: number };
  palette?: Palette;
  components: ThemeComponent[];
  wallpaper?: string;
}
export interface ThemeView { theme: Theme; origin: "official" | "imported" | "mine"; favorite: boolean; applied: boolean; compatible: boolean; files: string[] }
export interface PreviewItem {
  tool: string;
  name: string;
  selected: boolean;
  required: boolean;
  install: boolean;
  config_files: number;
  settings: number;
  activate: boolean;
  turn_off: string[];
  font: string | null;
  mods_skipped: number;
  incompatible: boolean;
}
export interface ThemePreview { theme_id: string; items: PreviewItem[]; wallpaper: boolean }
export interface ActiveTheme { id: string; name: string; accent: string | null; base: string | null; text: string | null }

export interface Snapshot {
  cpu: { model: string; usage: number; per_core: number[]; clock_mhz: number };
  ram: { used: number; total: number; percent: number };
  disks: { name: string; mount: string; used: number; total: number; read_bps: number; write_bps: number }[];
  net: { adapter: string; ip: string | null; down_bps: number; up_bps: number };
  system: { uptime_s: number; os: string; battery: { percent: number; charging: boolean } | null };
  gpus: string[];
  sensors: { available: boolean; reason: string; cpu_temp_c: number | null; gpu_temp_c: number | null };
  interval_s: number;
}
export interface PcInfo {
  maker: string;
  model: string;
  cpu: string;
  board: string;
  bios: string;
  windows: string;
  gpus: { name: string; driver_version: string; driver_date: string | null; driver_old: boolean }[];
}
export interface WidgetSettings {
  flyout: boolean;
  floating: boolean;
  close_to_tray: boolean;
  shortcut_enabled: boolean;
  shortcut: string;
  corner: string;
  opacity: number;
  layer: string;
  detailed: boolean;
  refresh_ms: number;
  metrics: string[];
  tools: string[];
  alert_ram_pct: number | null;
  alert_cpu_temp: number | null;
  alert_gpu_temp: number | null;
}
export interface WidgetSaved { settings: WidgetSettings; warning: string | null }
export interface Profile { name: string; tools: string[] }
export interface SwitchResult { outcome: Outcome; skipped: string[] }
export interface SyncSummary { tools_updated: string[]; themes_updated: string[]; errors: string[] }

export interface GlossaryTerm { id: string; term: I18nText; definition: I18nText }
export interface Issue { code: string; level: "error" | "warning" }
export interface PaletteChange { tool: string; name: string; key: string; role: string; color: string }
export interface PaletteResult { outcome: Outcome; changes: PaletteChange[] }
export interface MetaEdit { name: string; license: string; github: string; description_en: string; description_pt: string }

export type ToolAction = "install" | "uninstall" | "activate" | "deactivate";

export const api = {
  listTools: () => invoke<ToolView[]>("list_tools"),
  getDocs: (id: string, lang: string) => invoke<string | null>("get_docs", { id, lang }),
  planAction: (id: string, action: ToolAction, resolveConflicts = false) =>
    invoke<Plan>("plan_action", { id, action, resolveConflicts }),
  runAction: (id: string, action: ToolAction, resolveConflicts = false, tryMode = false) =>
    invoke<ActionResult>("run_action", { id, action, resolveConflicts, tryMode }),
  keepTry: (token: number) => invoke<boolean>("keep_try", { token }),
  revertNow: () => invoke<boolean>("revert_now"),
  systemInfo: () => invoke<SystemInfo>("system_info"),
  ackBuild: () => invoke<void>("ack_build"),
  getSettings: (id: string) => invoke<Record<string, unknown>>("get_settings", { id }),
  setSetting: (id: string, key: string, value: unknown) => invoke<Outcome>("set_setting", { id, key, value }),
  planRestore: (uninstall: boolean) => invoke<Plan>("plan_restore", { uninstall }),
  runRestore: (uninstall: boolean) => invoke<Outcome>("run_restore", { uninstall }),
  listHistory: () => invoke<HistoryEntry[]>("list_history"),
  listThemes: () => invoke<ThemeView[]>("list_themes"),
  themePreview: (id: string, selected: string[]) => invoke<ThemePreview>("theme_preview", { id, selected }),
  applyTheme: (id: string, selected: string[]) => invoke<Outcome>("apply_theme", { id, selected }),
  undoTheme: () => invoke<Outcome>("undo_theme"),
  activeTheme: () => invoke<ActiveTheme | null>("active_theme"),
  toggleFavorite: (id: string) => invoke<boolean>("toggle_favorite", { id }),
  importTheme: (url: string) => invoke<ThemeView>("import_theme", { url }),
  saveTheme: (id: string, name: string) => invoke<ThemeView>("save_theme", { id, name }),
  exportTheme: (id: string) => invoke<string>("export_theme", { id }),
  deleteTheme: (id: string) => invoke<void>("delete_theme", { id }),
  syncCatalog: () => invoke<SyncSummary>("sync_catalog"),
  undoHistory: (id: number) => invoke<Outcome>("undo_history", { id }),
  toggleToolFavorite: (id: string) => invoke<boolean>("toggle_tool_favorite", { id }),
  palettePreview: (id: string) => invoke<PaletteChange[]>("palette_preview", { id }),
  applyPalette: (id: string) => invoke<PaletteResult>("apply_palette", { id }),
  submissionCheck: (id: string) => invoke<Issue[]>("submission_check", { id }),
  editThemeMeta: (id: string, meta: MetaEdit) => invoke<ThemeView>("edit_theme_meta", { id, meta }),
  setThemeCover: (id: string, path: string) => invoke<ThemeView>("set_theme_cover", { id, path }),
  prepareSubmission: (id: string) => invoke<string>("prepare_submission", { id }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  getGlossary: () => invoke<GlossaryTerm[]>("get_glossary"),
  hardwareSnapshot: () => invoke<Snapshot>("hardware_snapshot"),
  pcInfo: () => invoke<PcInfo | null>("pc_info"),
  widgetSettings: () => invoke<WidgetSettings>("get_widget_settings"),
  setWidgetSettings: (settings: WidgetSettings) => invoke<WidgetSaved>("set_widget_settings", { settings }),
  setTrayLabels: (open: string, widgetLabel: string, quit: string) => invoke<void>("set_tray_labels", { open, widgetLabel, quit }),
  listProfiles: () => invoke<Profile[]>("list_profiles"),
  saveProfile: (name: string) => invoke<Profile>("save_profile", { name }),
  deleteProfile: (name: string) => invoke<void>("delete_profile", { name }),
  applyProfile: (name: string) => invoke<SwitchResult>("apply_profile", { name }),
  pause: () => invoke<SwitchResult>("pause_customizations"),
  resume: () => invoke<SwitchResult>("resume_customizations"),
  pausedState: () => invoke<string[] | null>("paused_state"),
  quickAction: (id: string, action: string) => invoke<void>("quick_action", { id, action }),
};

/** Text in the UI language, falling back to English (flag `missing` for the "not translated" badge). */
export function pick(text: I18nText | undefined, lang: string): { text: string; missing: boolean } {
  if (!text) return { text: "", missing: false };
  if (text[lang]) return { text: text[lang], missing: false };
  return { text: text.en ?? "", missing: lang !== "en" };
}
