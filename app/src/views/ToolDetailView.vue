<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { api, pick, type SystemInfo, type ToolView } from "@/api";
import { errorText } from "@/composables/operations";
import { renderMarkdown } from "@/markdown";
import { loadGlossary, matchers, terms } from "@/composables/glossary";
import { startAction, version } from "@/composables/operations";
import ToolToggle from "@/components/ToolToggle.vue";

const { t, locale } = useI18n();
const route = useRoute();
const id = computed(() => route.params.id as string);
const tool = ref<ToolView | null>(null);
const docs = ref("");
type Tab = "overview" | "settings" | "shortcuts" | "docs" | "compat";
const tab = ref<Tab>("overview");
const values = ref<Record<string, unknown>>({});
const system = ref<SystemInfo | null>(null);
const names = ref<Record<string, string>>({});
const settingsError = ref("");

async function load() {
  tool.value = (await api.listTools()).find((x) => x.manifest.id === id.value) ?? null;
  docs.value = (await api.getDocs(id.value, locale.value)) ?? "";
  system.value = await api.systemInfo();
  names.value = Object.fromEntries((await api.listTools()).map((x) => [x.manifest.id, x.manifest.name]));
  values.value = tool.value?.manifest.settings.length ? await api.getSettings(id.value) : {};
}

async function save(key: string, value: unknown) {
  settingsError.value = "";
  try {
    const out = await api.setSetting(id.value, key, value);
    if (!out.ok) settingsError.value = out.error ?? "";
    values.value = await api.getSettings(id.value);
  } catch (e) {
    settingsError.value = errorText(e);
  }
}
onMounted(() => {
  loadGlossary();
  load();
});
watch([id, locale, version], load);

const m = computed(() => tool.value?.manifest);
const tabs = computed<Tab[]>(() => {
  const list: Tab[] = ["overview"];
  if (m.value?.settings.length) list.push("settings");
  return [...list, "shortcuts", "docs", "compat"];
});
const html = computed(() => renderMarkdown(docs.value, terms.value.length ? matchers(locale.value) : []));
const risks = computed(() => pick(m.value?.risks, locale.value));
const undo = computed(() => pick(m.value?.how_to_undo, locale.value));
</script>

<template>
  <template v-if="tool && m">
    <router-link to="/tools" class="back">← {{ t("nav.tools") }}</router-link>
    <header class="header">
      <div>
        <h1>{{ m.name }}</h1>
        <p class="muted">{{ pick(m.summary, locale).text }}</p>
        <p class="muted small">
          {{ t("tool.license") }}: {{ m.license }} · <code>{{ m.homepage }}</code>
        </p>
        <p v-if="m.paid" class="warn">{{ t("tool.paidHint") }}</p>
        <p v-if="m.restricted_license" class="warn">{{ t("tool.restrictedHint") }}</p>
        <p v-if="!tool.compatible" class="bad" role="alert">{{ t("tool.incompatibleHint", { build: system?.build ?? "?" }) }}</p>
      </div>
      <div class="controls">
        <ToolToggle :tool="tool" />
        <button v-if="!tool.installed" class="btn" :disabled="!tool.compatible" @click="startAction(m.id, 'install', m.name)">{{ t("tool.install") }}</button>
        <button v-else class="btn secondary" @click="startAction(m.id, 'uninstall', m.name)">{{ t("tool.uninstall") }}</button>
      </div>
    </header>

    <div class="tabs" role="tablist">
      <button v-for="x in tabs" :key="x" role="tab" :aria-selected="tab === x" :class="{ on: tab === x }" @click="tab = x">
        {{ t("tab." + x) }}
      </button>
    </div>

    <section v-if="tab === 'overview'" class="card">
      <h2>{{ t("tool.changes") }}</h2>
      <ul>
        <li v-for="(c, i) in m.changes_on_screen" :key="i">{{ pick(c, locale).text }}</li>
      </ul>
      <h2 v-if="risks.text">{{ t("tool.risks") }}</h2>
      <p>{{ risks.text }}</p>
      <h2 v-if="undo.text">{{ t("tool.undo") }}</h2>
      <p>{{ undo.text }}</p>
      <p v-if="m.requires.admin" class="muted">{{ t("tool.adminHint") }}</p>
    </section>

    <section v-else-if="tab === 'settings'" class="card">
      <p class="muted">{{ t("tool.settingsHint") }}</p>
      <p v-if="settingsError" class="bad" role="alert">{{ settingsError }}</p>
      <div v-for="s in m.settings" :key="s.key" class="row">
        <label :for="'s-' + s.key">{{ pick(s.label, locale).text }}</label>
        <input v-if="s.type === 'number'" :id="'s-' + s.key" type="number" :min="s.min" :max="s.max"
               :value="values[s.key] as number" @change="save(s.key, Number(($event.target as HTMLInputElement).value))" />
        <input v-else-if="s.type === 'boolean'" :id="'s-' + s.key" type="checkbox"
               :checked="values[s.key] as boolean" @change="save(s.key, ($event.target as HTMLInputElement).checked)" />
        <input v-else :id="'s-' + s.key" :type="s.type === 'color' ? 'color' : 'text'"
               :value="values[s.key] as string" @change="save(s.key, ($event.target as HTMLInputElement).value)" />
      </div>
    </section>

    <section v-else-if="tab === 'docs'" class="card docs" v-html="html" />

    <section v-else-if="tab === 'compat'" class="card">
      <h2>{{ t("compat.windows") }}</h2>
      <p>
        {{ t("compat.yours", { build: system?.build ?? "?", edition: system?.edition ?? "" }) }}<br />
        {{ t("compat.needs", { min: m.requires.windows_min_build }) }}
        <template v-if="m.requires.windows_max_build"> {{ t("compat.max", { max: m.requires.windows_max_build }) }}</template>
      </p>
      <p :class="tool.compatible ? 'ok' : 'bad'">{{ tool.compatible ? t("compat.ok") : t("tool.incompatible") }}</p>
      <h2>{{ t("compat.conflicts") }}</h2>
      <p v-if="!m.conflicts.length" class="muted">{{ t("compat.none") }}</p>
      <ul v-else><li v-for="c in m.conflicts" :key="c"><router-link :to="'/tools/' + c">{{ names[c] ?? c }}</router-link></li></ul>
      <h2>{{ t("compat.pairs") }}</h2>
      <p v-if="!m.pairs_well_with.length" class="muted">{{ t("compat.none") }}</p>
      <ul v-else><li v-for="c in m.pairs_well_with" :key="c"><router-link :to="'/tools/' + c">{{ names[c] ?? c }}</router-link></li></ul>
    </section>

    <section v-else class="card">
      <p v-if="!m.shortcuts.length" class="muted">{{ t("tool.noShortcuts") }}</p>
      <table v-else class="table">
        <tr v-for="s in m.shortcuts" :key="s.keys">
          <td><kbd>{{ s.keys }}</kbd></td>
          <td>{{ pick(s, locale).text }}</td>
        </tr>
      </table>
    </section>
  </template>
  <p v-else class="muted">{{ t("common.loading") }}</p>
</template>

<style scoped>
.back { color: var(--accent); text-decoration: none; }
.header { display: flex; justify-content: space-between; gap: var(--space); align-items: flex-start; margin-top: 8px; }
.controls { display: flex; gap: 12px; align-items: center; }
.small { font-size: 0.9em; }
.warn { color: var(--warning); }
.bad { color: var(--danger); }
.ok { color: var(--success); }
.tabs { display: flex; gap: 4px; margin: var(--space) 0; border-bottom: 1px solid var(--border); }
.tabs button {
  font: inherit; color: var(--text-muted); background: none; border: 0; padding: 8px 14px; cursor: pointer;
  border-bottom: 2px solid transparent;
}
.tabs button.on { color: var(--text); border-bottom-color: var(--accent); }
.table td { padding: 6px 16px 6px 0; }
kbd { background: var(--surface); border: 1px solid var(--border); border-radius: 4px; padding: 2px 6px; }
.docs :deep(h2) { margin-top: 1.2em; }
.docs :deep(code), code { background: var(--surface); padding: 1px 5px; border-radius: 4px; }
</style>
