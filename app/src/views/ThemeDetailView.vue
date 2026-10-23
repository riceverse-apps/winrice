<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { api, pick, type PaletteChange, type ThemePreview, type ThemeView, type ToolView } from "@/api";
import SubmitWizard from "@/components/SubmitWizard.vue";
import { applyTheme, errorText, op, version } from "@/composables/operations";

const { t, locale } = useI18n();
const route = useRoute();
const router = useRouter();
const id = computed(() => route.params.id as string);
const view = ref<ThemeView | null>(null);
const tools = ref<Record<string, ToolView>>({});
const selected = ref<string[]>([]);
const preview = ref<ThemePreview | null>(null);
const reviewing = ref(false);
const confirmDelete = ref(false);
const changes = ref<PaletteChange[]>([]);
const syncing = ref(false);

async function load() {
  view.value = (await api.listThemes()).find((x) => x.theme.id === id.value) ?? null;
  tools.value = Object.fromEntries((await api.listTools()).map((x) => [x.manifest.id, x]));
  if (view.value && !selected.value.length) selected.value = view.value.theme.components.map((c) => c.tool);
  await refreshPreview();
  changes.value = view.value?.theme.palette ? await api.palettePreview(id.value).catch(() => []) : [];
}
async function refreshPreview() {
  preview.value = view.value ? await api.themePreview(id.value, selected.value) : null;
}
onMounted(load);
watch([id, version], () => {
  selected.value = [];
  reviewing.value = false;
  load();
});
watch(selected, refreshPreview);

const th = computed(() => view.value?.theme);
const missingRequired = computed(() => (th.value?.components ?? []).filter((c) => c.required && !selected.value.includes(c.tool)));
const blocked = computed(() => !selected.value.length || (preview.value?.items ?? []).some((i) => i.selected && i.incompatible && i.required));
const name = (tool: string) => tools.value[tool]?.manifest.name ?? tool;
const itemOf = (tool: string) => preview.value?.items.find((i) => i.tool === tool);
const colors = computed(() => Object.entries(th.value?.palette?.colors ?? {}));

function lines(tool: string): string[] {
  const i = itemOf(tool);
  if (!i) return [];
  const out: string[] = [];
  if (i.incompatible) out.push(t("themes.will.incompatible"));
  if (i.install) out.push(t("themes.will.install", { name: i.name }));
  if (i.config_files) out.push(t("themes.will.config", { n: i.config_files }));
  if (i.settings) out.push(t("themes.will.settings", { n: i.settings }));
  if (i.activate) out.push(t("themes.will.activate", { name: i.name }));
  if (i.turn_off.length) out.push(t("themes.will.turnOff", { names: i.turn_off.join(", ") }));
  if (i.font) out.push(t("themes.will.font", { font: i.font }));
  if (i.mods_skipped) out.push(t("themes.will.mods", { n: i.mods_skipped }));
  if (!out.length) out.push(t("themes.will.nothing"));
  return out;
}

async function apply() {
  reviewing.value = false;
  await applyTheme(id.value, selected.value);
}
async function favorite() {
  await api.toggleFavorite(id.value);
  version.value++;
}
async function exportZip() {
  try {
    const path = await api.exportTheme(id.value);
    op.notice = { kind: "ok", text: t("themes.exported", { path }) };
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  }
}
async function syncPalette() {
  syncing.value = true;
  op.notice = null;
  try {
    const r = await api.applyPalette(id.value);
    op.notice = r.outcome.ok
      ? { kind: "ok", text: t("themes.paletteDone", { n: r.changes.length }) }
      : { kind: "error", text: t("plan.failedRolledBack", { error: errorText(r.outcome.error ?? "") }) };
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  } finally {
    syncing.value = false;
    version.value++;
  }
}
async function remove() {
  await api.deleteTheme(id.value);
  version.value++;
  router.push("/themes");
}
</script>

<template>
  <template v-if="view && th">
    <router-link to="/themes" class="back">← {{ t("nav.themes") }}</router-link>
    <header class="header">
      <div>
        <h1>{{ th.name }}</h1>
        <p class="muted">{{ pick(th.description, locale).text }}</p>
        <p class="muted small">
          {{ t("themes.by", { author: th.author.name }) }} · v{{ th.version }} · {{ th.license }}
          <template v-if="th.author.github"> · <code>github.com/{{ th.author.github }}</code></template>
        </p>
        <p v-if="view.origin === 'imported'" class="warn" role="alert">{{ t("themes.unverifiedHint") }}</p>
        <p v-if="!view.compatible" class="bad" role="alert">{{ t("themes.incompatibleHint", { build: th.requires?.windows_min_build ?? "?" }) }}</p>
      </div>
      <div class="controls">
        <button class="btn secondary" :aria-pressed="view.favorite" @click="favorite">{{ view.favorite ? "★" : "☆" }} {{ t("themes.favorite") }}</button>
        <button class="btn secondary" @click="exportZip">{{ t("themes.export") }}</button>
        <button v-if="view.origin !== 'official'" class="btn secondary" @click="confirmDelete = true">{{ t("themes.delete") }}</button>
      </div>
    </header>

    <section v-if="confirmDelete" class="card" role="alertdialog">
      <p>{{ t("themes.deleteConfirm", { name: th.name }) }}</p>
      <button class="btn secondary" @click="confirmDelete = false">{{ t("common.cancel") }}</button>
      <button class="btn" @click="remove">{{ t("themes.delete") }}</button>
    </section>

    <section v-if="colors.length" class="card">
      <h2>{{ t("themes.palette", { name: th.palette?.name }) }}</h2>
      <div class="colors">
        <span v-for="[k, c] in colors" :key="k" class="color"><i :style="{ background: c }" />{{ k }} <code>{{ c }}</code></span>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("themes.components") }}</h2>
      <p class="muted">{{ t("themes.componentsHint") }}</p>
      <div v-for="c in th.components" :key="c.tool" class="component">
        <label>
          <input v-model="selected" type="checkbox" :value="c.tool" />
          <router-link :to="'/tools/' + c.tool">{{ name(c.tool) }}</router-link>
          <span class="badge">{{ c.required ? t("themes.required") : t("themes.optional") }}</span>
          <span v-if="tools[c.tool]?.manifest.paid" class="badge warn">{{ t("tool.paid") }}</span>
          <span v-if="tools[c.tool]?.manifest.restricted_license" class="badge warn">{{ t("tool.restricted") }}</span>
        </label>
        <ul v-if="selected.includes(c.tool)" class="muted">
          <li v-for="(l, i) in lines(c.tool)" :key="i">{{ l }}</li>
        </ul>
      </div>
      <p v-if="missingRequired.length" class="warn" role="alert">
        {{ t("themes.missingRequired", { names: missingRequired.map((c) => name(c.tool)).join(", ") }) }}
      </p>
      <p v-if="preview?.wallpaper" class="muted">{{ t("themes.will.wallpaper") }}</p>
    </section>

    <section v-if="colors.length" class="card">
      <h2>{{ t("themes.syncTitle") }}</h2>
      <p class="muted">{{ t("themes.syncText") }}</p>
      <ul v-if="changes.length">
        <li v-for="c in changes" :key="c.tool + c.key">
          {{ t("themes.syncItem", { tool: c.name, role: c.role, color: c.color }) }}
        </li>
      </ul>
      <p v-else class="muted">{{ t("themes.syncNone") }}</p>
      <button class="btn secondary" :disabled="!changes.length || syncing || op.busy" @click="syncPalette">{{ t("themes.syncButton") }}</button>
    </section>

    <SubmitWizard v-if="view.origin !== 'official'" :view="view" />

    <section class="card">
      <template v-if="!reviewing">
        <button class="btn" :disabled="blocked || op.busy || !view.compatible" @click="reviewing = true">{{ t("themes.review") }}</button>
        <button v-if="view.applied" class="btn secondary" :disabled="op.busy" @click="$router.push('/')">{{ t("themes.appliedGoHome") }}</button>
      </template>
      <template v-else>
        <h2>{{ t("themes.reviewTitle") }}</h2>
        <p class="muted">{{ t("themes.reviewHint") }}</p>
        <button class="btn secondary" :disabled="op.busy" @click="reviewing = false">{{ t("common.cancel") }}</button>
        <button class="btn" :disabled="op.busy" @click="apply">{{ op.busy ? t("common.working") : t("themes.apply") }}</button>
      </template>
    </section>
  </template>
  <p v-else class="muted">{{ t("common.loading") }}</p>
</template>

<style scoped>
.back { color: var(--accent); text-decoration: none; }
.header { display: flex; justify-content: space-between; gap: var(--space); align-items: flex-start; margin-top: 8px; }
.controls { display: flex; gap: 8px; flex-wrap: wrap; }
.small { font-size: 0.9em; }
.warn { color: var(--warning); }
.bad { color: var(--danger); }
.colors { display: flex; gap: 16px; flex-wrap: wrap; }
.color { display: inline-flex; align-items: center; gap: 6px; }
.color i { width: 20px; height: 20px; border-radius: 50%; border: 1px solid var(--border); display: inline-block; }
.component { padding: 6px 0; border-bottom: 1px solid var(--border); }
.component label { display: flex; align-items: center; gap: 8px; }
.component ul { margin: 4px 0 4px 32px; }
.badge { font-size: 0.8em; padding: 2px 8px; border: 1px solid var(--border); border-radius: 99px; color: var(--text-muted); }
.badge.warn { color: var(--warning); border-color: var(--warning); }
code { background: var(--surface); padding: 1px 5px; border-radius: 4px; }
</style>
