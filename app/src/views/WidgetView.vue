<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";
import { api, pick, type ActiveTheme, type Profile, type Snapshot, type ToolView, type WidgetSettings } from "@/api";
import { errorText, version } from "@/composables/operations";
import { bps, bytes, clock, duration } from "@/format";
import Spark from "@/components/Spark.vue";

const { t, locale } = useI18n();
const label = (() => {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
})();
const floating = label === "widget";
const standalone = label === "widget" || label === "flyout";

const settings = ref<WidgetSettings | null>(null);
const tools = ref<ToolView[]>([]);
const profiles = ref<Profile[]>([]);
const paused = ref<string[] | null>(null);
const theme = ref<ActiveTheme | null>(null);
const snap = ref<Snapshot | null>(null);
const message = ref<{ kind: "ok" | "error"; text: string } | null>(null);
const busy = ref<string | null>(null);
const confirming = ref<string | null>(null);
const conflict = ref<{ id: string; names: string[] } | null>(null);
const confirmRestore = ref(false);
const profile = ref("");

const HISTORY = 60;
const hist = ref({ cpu: [] as number[], ram: [] as number[], down: [] as number[], up: [] as number[] });
const push = (a: number[], v: number) => {
  a.push(v);
  if (a.length > HISTORY) a.shift();
};

async function loadAll() {
  const [s, tl, pr, pa, th] = await Promise.all([api.widgetSettings(), api.listTools(), api.listProfiles(), api.pausedState(), api.activeTheme()]);
  settings.value = s;
  tools.value = tl;
  profiles.value = pr;
  paused.value = pa;
  theme.value = th;
}

// ---- hardware polling: only while this window is visible; refresh rate from settings
let timer: number | undefined;
let stopped = false;
async function poll() {
  if (stopped) return;
  if (!document.hidden && settings.value) {
    try {
      const s = await api.hardwareSnapshot();
      snap.value = s;
      push(hist.value.cpu, s.cpu.usage);
      push(hist.value.ram, s.ram.percent);
      push(hist.value.down, s.net.down_bps);
      push(hist.value.up, s.net.up_bps);
    } catch {
      /* the next tick tries again */
    }
  }
  timer = window.setTimeout(poll, Math.max(500, settings.value?.refresh_ms ?? 1000));
}

let unlisten: Array<() => void> = [];
onMounted(async () => {
  document.documentElement.classList.toggle("widget-window", standalone);
  await loadAll();
  poll();
  unlisten.push(await listen<WidgetSettings>("winrice://widget-settings", (e) => (settings.value = e.payload)));
  unlisten.push(await listen("winrice://reverted", loadAll));
});
onBeforeUnmount(() => {
  stopped = true;
  window.clearTimeout(timer);
  unlisten.forEach((u) => u());
});
watch(version, loadAll);

// ---- tools
const shown = computed(() => {
  const s = settings.value;
  const usable = tools.value.filter((x) => x.installed && x.toggleable && x.compatible);
  if (s?.tools.length) {
    return s.tools.map((id) => usable.find((x) => x.manifest.id === id)).filter((x): x is ToolView => !!x);
  }
  return usable.filter((x) => x.manifest.widget?.show_by_default !== false);
});
const needsCare = (x: ToolView) => x.manifest.requires.admin || !!x.manifest.activate?.autostart_task || ["taskbar-start", "shell-mod"].includes(x.manifest.category);
const clashing = (x: ToolView) =>
  tools.value.filter((o) => o.active && o.manifest.id !== x.manifest.id && (x.manifest.conflicts.includes(o.manifest.id) || o.manifest.conflicts.includes(x.manifest.id)));

function ask(x: ToolView) {
  conflict.value = null;
  if (needsCare(x) && confirming.value !== x.manifest.id) {
    confirming.value = x.manifest.id;
    return;
  }
  confirming.value = null;
  toggle(x, false);
}

async function toggle(x: ToolView, resolve: boolean) {
  const id = x.manifest.id;
  busy.value = id;
  message.value = null;
  conflict.value = null;
  try {
    const res = await api.runAction(id, x.active ? "deactivate" : "activate", resolve, false);
    if (!res.outcome.ok) message.value = { kind: "error", text: t("plan.failedRolledBack", { error: errorText(res.outcome.error ?? "") }) };
  } catch (e) {
    const [code, arg = ""] = String(e).split(":");
    if (code === "conflict") {
      conflict.value = { id, names: arg.split(",").map((i) => tools.value.find((o) => o.manifest.id === i)?.manifest.name ?? i) };
    } else {
      message.value = { kind: "error", text: errorText(e) };
    }
  } finally {
    busy.value = null;
    version.value++;
  }
}

async function quick(x: ToolView, action: string) {
  try {
    await api.quickAction(x.manifest.id, action);
    message.value = { kind: "ok", text: t("widget.done") };
  } catch (e) {
    message.value = { kind: "error", text: errorText(e) };
  }
}

// ---- profiles, pause, restore
async function run(fn: () => Promise<{ outcome: { ok: boolean; error: string | null }; skipped: string[] }>, ok: string) {
  busy.value = "bulk";
  message.value = null;
  try {
    const r = await fn();
    message.value = r.outcome.ok
      ? { kind: "ok", text: r.skipped.length ? t("widget.doneSkipped", { names: r.skipped.join(", ") }) : ok }
      : { kind: "error", text: t("plan.failedRolledBack", { error: errorText(r.outcome.error ?? "") }) };
  } catch (e) {
    message.value = { kind: "error", text: errorText(e) };
  } finally {
    busy.value = null;
    version.value++;
  }
}
const applyProfile = () => profile.value && run(() => api.applyProfile(profile.value), t("widget.done"));
const pause = () => run(() => api.pause(), t("widget.paused"));
const resume = () => run(() => api.resume(), t("widget.resumed"));
async function restore() {
  confirmRestore.value = false;
  busy.value = "bulk";
  try {
    const out = await api.runRestore(false);
    message.value = out.ok ? { kind: "ok", text: t("plan.done") } : { kind: "error", text: errorText(out.error ?? "") };
  } catch (e) {
    message.value = { kind: "error", text: errorText(e) };
  } finally {
    busy.value = null;
    version.value++;
  }
}

// ---- display
const has = (m: string) => settings.value?.metrics.includes(m) ?? false;
const detailed = computed(() => settings.value?.detailed ?? false);
const alerts = computed(() => {
  const out: string[] = [];
  const s = settings.value;
  if (!s || !snap.value) return out;
  if (s.alert_ram_pct && snap.value.ram.percent >= s.alert_ram_pct) out.push(t("widget.alert.ram", { n: Math.round(snap.value.ram.percent) }));
  const ct = snap.value.sensors.cpu_temp_c;
  if (s.alert_cpu_temp && ct != null && ct >= s.alert_cpu_temp) out.push(t("widget.alert.cpuTemp", { n: Math.round(ct) }));
  const gt = snap.value.sensors.gpu_temp_c;
  if (s.alert_gpu_temp && gt != null && gt >= s.alert_gpu_temp) out.push(t("widget.alert.gpuTemp", { n: Math.round(gt) }));
  return out;
});
const rootStyle = computed(() => {
  const th = theme.value;
  const style: Record<string, string> = {};
  if (th?.accent) style["--accent"] = th.accent;
  if (th?.base) style.background = th.base;
  if (th?.text) {
    style.color = th.text;
    style["--text"] = th.text;
  }
  if (floating && settings.value) style.opacity = String(settings.value.opacity);
  return style;
});

const closeSelf = () => invoke("hide_window", { label });
const openMain = () => invoke("open_main_window");
</script>

<template>
  <div class="widget" :class="{ float: floating }" :style="rootStyle">
    <header class="top" data-tauri-drag-region>
      <strong data-tauri-drag-region>WinRice</strong>
      <span class="spacer" data-tauri-drag-region />
      <button v-if="standalone" class="ghost" :title="t('widget.openMain')" :aria-label="t('widget.openMain')" @click="openMain">⧉</button>
      <button v-if="standalone" class="ghost" :title="t('widget.close')" :aria-label="t('widget.close')" @click="closeSelf">✕</button>
    </header>

    <p v-if="message" class="msg" :class="message.kind" :role="message.kind === 'error' ? 'alert' : 'status'" @click="message = null">{{ message.text }}</p>
    <p v-for="a in alerts" :key="a" class="msg error" role="alert">⚠ {{ a }}</p>

    <!-- quick toggles -->
    <section>
      <h3>{{ t("widget.tools") }}</h3>
      <p v-if="!shown.length" class="muted">{{ t("widget.noTools") }}</p>
      <div v-for="x in shown" :key="x.manifest.id" class="tool">
        <div class="line">
          <span class="name">
            {{ x.manifest.name }}
            <span v-if="needsCare(x)" class="warn" :title="t('widget.careHint')">⚠</span>
          </span>
          <span class="state" :class="{ on: x.active }">
            {{ clashing(x).length && !x.active ? t("widget.state.conflict") : x.active ? t("widget.state.active") : t("widget.state.off") }}
          </span>
          <button
            class="switch"
            role="switch"
            :aria-checked="x.active"
            :aria-label="t('tool.toggle', { name: x.manifest.name })"
            :disabled="busy !== null"
            @click="ask(x)"
          >
            <span class="knob" />
          </button>
        </div>
        <div v-if="confirming === x.manifest.id" class="confirm" role="alert">
          {{ t("widget.confirmCare", { name: x.manifest.name }) }}
          <button class="btn small secondary" @click="confirming = null">{{ t("common.cancel") }}</button>
          <button class="btn small" @click="ask(x)">{{ t("common.confirm") }}</button>
        </div>
        <div v-if="conflict?.id === x.manifest.id" class="confirm" role="alert">
          {{ t("conflict.text", { name: x.manifest.name, others: conflict.names.join(", ") }) }}
          <button class="btn small" @click="toggle(x, true)">{{ t("conflict.resolve", { others: conflict.names.join(", ") }) }}</button>
        </div>
        <div v-if="x.active && x.manifest.widget?.quick_actions.length" class="actions">
          <button v-for="q in x.manifest.widget.quick_actions" :key="q.id" class="btn small secondary" @click="quick(x, q.id)">
            {{ pick(q.label, locale).text }}
          </button>
        </div>
      </div>
    </section>

    <!-- profiles / pause / restore -->
    <section>
      <h3>{{ t("widget.profiles") }}</h3>
      <div class="line">
        <select v-model="profile" :aria-label="t('widget.profiles')" class="grow">
          <option value="" disabled>{{ profiles.length ? t("widget.chooseProfile") : t("widget.noProfiles") }}</option>
          <option v-for="p in profiles" :key="p.name" :value="p.name">{{ p.name }}</option>
        </select>
        <button class="btn small" :disabled="!profile || busy !== null" @click="applyProfile">{{ t("widget.apply") }}</button>
      </div>
      <div class="line wrap">
        <button v-if="!paused" class="btn small secondary" :disabled="busy !== null" @click="pause">⏸ {{ t("widget.pause") }}</button>
        <button v-else class="btn small" :disabled="busy !== null" @click="resume">▶ {{ t("widget.resume") }}</button>
        <button v-if="!confirmRestore" class="btn small secondary" :disabled="busy !== null" @click="confirmRestore = true">{{ t("home.restore") }}</button>
        <span v-else class="confirm">
          {{ t("widget.confirmRestore") }}
          <button class="btn small secondary" @click="confirmRestore = false">{{ t("common.cancel") }}</button>
          <button class="btn small" @click="restore">{{ t("common.confirm") }}</button>
        </span>
      </div>
      <p v-if="paused" class="muted">{{ t("widget.pausedHint", { n: paused.length }) }}</p>
    </section>

    <!-- hardware -->
    <section v-if="snap">
      <h3>{{ t("widget.hardware") }}</h3>

      <div v-if="has('cpu')" class="metric">
        <div class="line"><span>CPU</span><b>{{ snap.cpu.usage.toFixed(0) }}%</b></div>
        <div class="bar"><i :style="{ width: snap.cpu.usage + '%' }" /></div>
        <p class="muted small">{{ snap.cpu.model }} · {{ clock(snap.cpu.clock_mhz) }}</p>
        <template v-if="detailed">
          <Spark :values="hist.cpu" :max="100" />
          <div class="cores"><i v-for="(c, i) in snap.cpu.per_core" :key="i" :style="{ height: Math.max(2, c * 0.28) + 'px' }" :title="`#${i} ${c.toFixed(0)}%`" /></div>
        </template>
      </div>

      <div v-if="has('gpu')" class="metric">
        <div class="line"><span>GPU</span><b class="muted">{{ t("widget.needsHelper") }}</b></div>
        <p class="muted small">{{ snap.gpus.join(" · ") || t("widget.unknown") }}</p>
      </div>

      <div v-if="has('ram')" class="metric">
        <div class="line"><span>RAM</span><b>{{ bytes(snap.ram.used) }} / {{ bytes(snap.ram.total) }}</b></div>
        <div class="bar"><i :style="{ width: snap.ram.percent + '%' }" /></div>
        <Spark v-if="detailed" :values="hist.ram" :max="100" />
      </div>

      <div v-if="has('disk')" class="metric">
        <div v-for="d in snap.disks" :key="d.mount" class="disk">
          <div class="line"><span>{{ d.mount }}</span><b>{{ bytes(d.used) }} / {{ bytes(d.total) }}</b></div>
          <div class="bar"><i :style="{ width: (d.total ? (d.used * 100) / d.total : 0) + '%' }" /></div>
          <p class="muted small">↓ {{ bps(d.read_bps) }} · ↑ {{ bps(d.write_bps) }}</p>
        </div>
      </div>

      <div v-if="has('net')" class="metric">
        <div class="line"><span>{{ t("widget.network") }}</span><b>↓ {{ bps(snap.net.down_bps) }} ↑ {{ bps(snap.net.up_bps) }}</b></div>
        <p class="muted small">{{ snap.net.adapter }} {{ snap.net.ip }}</p>
        <template v-if="detailed"><Spark :values="hist.down" /><Spark :values="hist.up" /></template>
      </div>

      <div v-if="has('system')" class="metric">
        <div class="line"><span>{{ t("widget.system") }}</span><b>{{ t("widget.uptime", { t: duration(snap.system.uptime_s) }) }}</b></div>
        <p class="muted small">{{ snap.system.os }}</p>
        <p v-if="snap.system.battery" class="muted small">🔋 {{ snap.system.battery.percent }}% {{ snap.system.battery.charging ? "⚡" : "" }}</p>
      </div>

      <div v-if="has('sensors')" class="metric">
        <div class="line"><span>{{ t("widget.sensors") }}</span></div>
        <p class="muted small">{{ t("widget.sensorsHint") }}</p>
      </div>
    </section>
    <p v-else class="muted">{{ t("common.loading") }}</p>
  </div>
</template>

<style scoped>
.widget {
  background: var(--bg-elevated); color: var(--text); border: 1px solid var(--border);
  border-radius: 12px; padding: 12px; font-size: 0.92em; height: 100vh; overflow: auto;
}
.widget:not(.float) { border-radius: 0; }
.top { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; user-select: none; }
.spacer { flex: 1; }
.ghost { background: none; border: 0; color: var(--text-muted); cursor: pointer; font-size: 1em; padding: 4px 6px; border-radius: 6px; }
.ghost:hover { background: var(--surface); }
h3 { margin: 12px 0 6px; font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); }
.line { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 3px 0; }
.line.wrap { flex-wrap: wrap; justify-content: flex-start; }
.grow { flex: 1; min-width: 0; }
.name { font-weight: 600; }
.warn { color: var(--warning); }
.state { margin-left: auto; font-size: 0.8em; color: var(--text-muted); }
.state.on { color: var(--success); }
.small { font-size: 0.85em; margin: 2px 0; }
.btn.small { padding: 4px 10px; font-size: 0.9em; }
.confirm { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; font-size: 0.9em; padding: 4px 0; }
.actions { display: flex; gap: 6px; flex-wrap: wrap; padding: 2px 0 4px; }
.msg { margin: 6px 0; padding: 6px 10px; border-radius: 8px; border: 1px solid var(--border); cursor: pointer; }
.msg.ok { border-color: var(--success); }
.msg.error { border-color: var(--danger); }
.switch { width: 40px; height: 22px; border-radius: 11px; border: 1px solid var(--border); background: var(--surface); position: relative; cursor: pointer; padding: 0; flex: none; }
.switch[aria-checked="true"] { background: var(--accent); border-color: var(--accent); }
.switch:disabled { opacity: 0.5; cursor: wait; }
.knob { position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: var(--text-muted); transition: transform 0.15s; }
.switch[aria-checked="true"] .knob { transform: translateX(18px); background: var(--accent-text); }
.metric { padding: 6px 0; border-bottom: 1px solid var(--border); }
.bar { height: 6px; background: var(--surface); border-radius: 3px; overflow: hidden; }
.bar i { display: block; height: 100%; background: var(--accent); transition: width 0.3s; }
.cores { display: flex; align-items: flex-end; gap: 2px; height: 30px; margin-top: 4px; }
.cores i { flex: 1; background: var(--accent); border-radius: 1px; min-width: 2px; }
</style>
