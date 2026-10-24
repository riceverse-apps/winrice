<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type ToolView, type WidgetSettings } from "@/api";
import { errorText } from "@/composables/operations";

const { t } = useI18n();
const s = ref<WidgetSettings | null>(null);
const tools = ref<ToolView[]>([]);
const warning = ref("");
const METRICS = ["cpu", "gpu", "ram", "disk", "net", "system", "pc", "sensors"];

onMounted(async () => {
  s.value = await api.widgetSettings();
  tools.value = await api.listTools();
});

async function save() {
  if (!s.value) return;
  const res = await api.setWidgetSettings(s.value);
  s.value = res.settings;
  warning.value = res.warning ? errorText(res.warning) : "";
}

const usable = computed(() => tools.value.filter((x) => x.installed && x.toggleable));
/** Tools shown in the widget, in order; empty list in settings means "all of them". */
const order = computed(() => (s.value?.tools.length ? s.value.tools : usable.value.map((x) => x.manifest.id)));
const nameOf = (id: string) => tools.value.find((x) => x.manifest.id === id)?.manifest.name ?? id;

function setTools(list: string[]) {
  if (!s.value) return;
  s.value.tools = list;
  save();
}
function toggleTool(id: string, on: boolean) {
  setTools(on ? [...order.value, id] : order.value.filter((x) => x !== id));
}
function move(id: string, d: -1 | 1) {
  const list = [...order.value];
  const i = list.indexOf(id);
  const j = i + d;
  if (i < 0 || j < 0 || j >= list.length) return;
  [list[i], list[j]] = [list[j], list[i]];
  setTools(list);
}
function toggleMetric(m: string, on: boolean) {
  if (!s.value) return;
  s.value.metrics = on ? [...s.value.metrics, m] : s.value.metrics.filter((x) => x !== m);
  save();
}
function num(v: string): number | null {
  const n = Number(v);
  return v.trim() === "" || !Number.isFinite(n) ? null : n;
}
</script>

<template>
  <section v-if="s" class="card">
    <h2>{{ t("widgetCfg.title") }}</h2>
    <p class="muted">{{ t("widgetCfg.text") }}</p>
    <p v-if="warning" class="bad" role="alert">{{ warning }}</p>

    <div class="row"><label for="w-fly">{{ t("widgetCfg.flyout") }}</label><input id="w-fly" v-model="s.flyout" type="checkbox" @change="save" /></div>
    <div class="row"><label for="w-float">{{ t("widgetCfg.floating") }}</label><input id="w-float" v-model="s.floating" type="checkbox" @change="save" /></div>
    <div class="row"><label for="w-tray">{{ t("widgetCfg.closeToTray") }}</label><input id="w-tray" v-model="s.close_to_tray" type="checkbox" @change="save" /></div>
    <div class="row">
      <label for="w-sc">{{ t("widgetCfg.shortcut") }}</label>
      <span>
        <input v-model="s.shortcut_enabled" type="checkbox" :aria-label="t('widgetCfg.shortcutOn')" @change="save" />
        <input id="w-sc" v-model="s.shortcut" :disabled="!s.shortcut_enabled" placeholder="Super+Alt+W" @change="save" />
      </span>
    </div>

    <div class="row">
      <label for="w-corner">{{ t("widgetCfg.corner") }}</label>
      <select id="w-corner" v-model="s.corner" @change="save">
        <option v-for="c in ['top-left', 'top-right', 'bottom-left', 'bottom-right']" :key="c" :value="c">{{ t("widgetCfg.corners." + c) }}</option>
      </select>
    </div>
    <div class="row">
      <label for="w-layer">{{ t("widgetCfg.layer") }}</label>
      <select id="w-layer" v-model="s.layer" @change="save">
        <option v-for="c in ['top', 'normal', 'bottom']" :key="c" :value="c">{{ t("widgetCfg.layers." + c) }}</option>
      </select>
    </div>
    <div class="row">
      <label for="w-op">{{ t("widgetCfg.opacity") }}</label>
      <input id="w-op" v-model.number="s.opacity" type="range" min="0.3" max="1" step="0.05" @change="save" />
    </div>
    <div class="row"><label for="w-det">{{ t("widgetCfg.detailed") }}</label><input id="w-det" v-model="s.detailed" type="checkbox" @change="save" /></div>
    <div class="row">
      <label for="w-ms">{{ t("widgetCfg.refresh") }}</label>
      <input id="w-ms" v-model.number="s.refresh_ms" type="number" min="500" step="100" @change="save" />
    </div>

    <h2>{{ t("widgetCfg.metrics") }}</h2>
    <label v-for="m in METRICS" :key="m" class="chk">
      <input type="checkbox" :checked="s.metrics.includes(m)" @change="toggleMetric(m, ($event.target as HTMLInputElement).checked)" />
      {{ t("widgetCfg.metric." + m) }}
    </label>

    <h2>{{ t("widgetCfg.tools") }}</h2>
    <p class="muted">{{ t("widgetCfg.toolsHint") }}</p>
    <div v-for="id in order" :key="id" class="row">
      <label><input type="checkbox" checked @change="toggleTool(id, false)" /> {{ nameOf(id) }}</label>
      <span>
        <button class="btn secondary" :aria-label="t('widgetCfg.up')" @click="move(id, -1)">↑</button>
        <button class="btn secondary" :aria-label="t('widgetCfg.down')" @click="move(id, 1)">↓</button>
      </span>
    </div>
    <label v-for="x in usable.filter((u) => !order.includes(u.manifest.id))" :key="x.manifest.id" class="chk">
      <input type="checkbox" @change="toggleTool(x.manifest.id, true)" /> {{ x.manifest.name }}
    </label>

    <h2>{{ t("widgetCfg.alerts") }}</h2>
    <div class="row">
      <label for="a-ram">{{ t("widgetCfg.alertRam") }}</label>
      <input id="a-ram" type="number" min="1" max="100" :value="s.alert_ram_pct ?? ''" @change="s.alert_ram_pct = num(($event.target as HTMLInputElement).value); save()" />
    </div>
    <div class="row">
      <label for="a-cpu">{{ t("widgetCfg.alertCpuTemp") }}</label>
      <input id="a-cpu" type="number" min="30" max="120" :value="s.alert_cpu_temp ?? ''" @change="s.alert_cpu_temp = num(($event.target as HTMLInputElement).value); save()" />
    </div>
    <div class="row">
      <label for="a-gpu">{{ t("widgetCfg.alertGpuTemp") }}</label>
      <input id="a-gpu" type="number" min="30" max="120" :value="s.alert_gpu_temp ?? ''" @change="s.alert_gpu_temp = num(($event.target as HTMLInputElement).value); save()" />
    </div>
    <p class="muted">{{ t("widgetCfg.tempHint") }}</p>
  </section>
</template>

<style scoped>
.chk { display: inline-flex; align-items: center; gap: 6px; margin-right: 16px; padding: 4px 0; }
.bad { color: var(--danger); }
input[type="number"], input:not([type]) {
  font: inherit; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 8px; width: 140px;
}
</style>
