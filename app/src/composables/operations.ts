import { reactive, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { api, type Outcome, type Plan, type Reverted, type ToolAction } from "@/api";
import { i18n } from "@/i18n";

const t = (k: string, p?: Record<string, unknown>) => i18n.global.t(k, p ?? {}) as string;

export const op = reactive({
  plan: null as Plan | null,
  title: "",
  busy: false,
  /** Can this plan be tried for 30 s before keeping it? */
  canTry: false,
  notice: null as { kind: "ok" | "error"; text: string } | null,
  /** A conflict the user can resolve by turning the other tools off. */
  conflict: null as { id: string; name: string; others: string[] } | null,
  /** A change on probation: reverts by itself at `deadline` unless kept. */
  trial: null as { token: number; name: string; deadline: number } | null,
});
let runner: ((tryMode: boolean) => Promise<{ outcome: Outcome; token: number | null; try_seconds: number | null }>) | null = null;
let current: { id: string; name: string } | null = null;

/** Bumped after every finished operation so lists reload. */
export const version = ref(0);

/** Turns a backend error code ("conflict:a,b", "not_installed", ...) into a readable message. */
export function errorText(raw: unknown): string {
  const s = String(raw);
  const [code, ...rest] = s.split(":");
  const arg = rest.join(":");
  const key = "errors." + code;
  return i18n.global.te(key) ? t(key, { ids: arg.split(",").join(", "), method: arg }) : s;
}

function fail(e: unknown) {
  op.notice = { kind: "error", text: errorText(e) };
}

async function toolName(id: string): Promise<string> {
  return (await api.listTools()).find((x) => x.manifest.id === id)?.manifest.name ?? id;
}

export async function startAction(id: string, action: ToolAction, name: string, resolveConflicts = false) {
  op.notice = null;
  op.conflict = null;
  try {
    op.plan = await api.planAction(id, action, resolveConflicts);
    op.title = t("plan.title." + action, { name });
    op.canTry = action === "activate";
    current = { id, name };
    runner = (tryMode) => api.runAction(id, action, resolveConflicts, tryMode);
  } catch (e) {
    const [code, arg = ""] = String(e).split(":");
    if (code === "conflict") {
      const names = await Promise.all(arg.split(",").map(toolName));
      op.conflict = { id, name, others: names };
    } else fail(e);
  }
}

export async function startRestore(uninstall: boolean) {
  op.notice = null;
  try {
    op.plan = await api.planRestore(uninstall);
    op.title = t("plan.title.restore");
    op.canTry = false;
    runner = async () => ({ outcome: await api.runRestore(uninstall), token: null, try_seconds: null });
  } catch (e) {
    fail(e);
  }
}

export function cancel() {
  op.plan = null;
  runner = null;
}

export async function confirm(tryMode = false) {
  if (!runner) return;
  op.busy = true;
  try {
    const res = await runner(tryMode);
    if (!res.outcome.ok) {
      op.notice = { kind: "error", text: t("plan.failedRolledBack", { error: res.outcome.error ?? "" }) };
    } else if (res.token !== null && res.try_seconds !== null) {
      op.trial = { token: res.token, name: current?.name ?? "", deadline: Date.now() + res.try_seconds * 1000 };
    } else {
      op.notice = { kind: "ok", text: t("plan.done") };
    }
  } catch (e) {
    fail(e);
  } finally {
    op.busy = false;
    op.plan = null;
    runner = null;
    version.value++;
  }
}

export async function applyTheme(id: string, selected: string[]) {
  op.notice = null;
  op.busy = true;
  try {
    const out = await api.applyTheme(id, selected);
    op.notice = out.ok
      ? { kind: "ok", text: t("themes.done") }
      : { kind: "error", text: t("plan.failedRolledBack", { error: errorText(out.error ?? "") }) };
  } catch (e) {
    fail(e);
  } finally {
    op.busy = false;
    version.value++;
  }
}

export async function undoTheme() {
  op.notice = null;
  op.busy = true;
  try {
    const out = await api.undoTheme();
    op.notice = out.ok ? { kind: "ok", text: t("themes.undone") } : { kind: "error", text: errorText(out.error ?? "") };
  } catch (e) {
    fail(e);
  } finally {
    op.busy = false;
    version.value++;
  }
}

export async function keepTrial() {
  if (!op.trial) return;
  const token = op.trial.token;
  op.trial = null;
  await api.keepTry(token);
  op.notice = { kind: "ok", text: t("trial.kept") };
  version.value++;
}

export async function revertTrial() {
  op.trial = null;
  await api.revertNow();
}

/** Backend reverted something by itself (timer, shortcut, watchdog). */
export function listenForReverts() {
  return listen<Reverted>("winrice://reverted", (e) => {
    op.trial = null;
    op.notice = { kind: "ok", text: t("trial.reverted." + e.payload.reason, { tool: e.payload.tool }) };
    version.value++;
  });
}
