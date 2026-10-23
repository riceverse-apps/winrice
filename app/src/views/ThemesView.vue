<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, pick, type ThemeView } from "@/api";
import { errorText, op, version } from "@/composables/operations";

const { t, locale } = useI18n();
type Tab = "official" | "imported" | "mine" | "favorites";
const tab = ref<Tab>("official");
const themes = ref<ThemeView[]>([]);
const query = ref("");
const tag = ref("");
const difficulty = ref("");
const onlyCompatible = ref(false);
const importUrl = ref("");
const saveName = ref("");
const busy = ref(false);

async function load() {
  themes.value = await api.listThemes();
}
onMounted(load);
watch(version, load);

const tags = computed(() => [...new Set(themes.value.flatMap((x) => x.theme.tags))].sort());
const shown = computed(() => {
  const q = query.value.trim().toLowerCase();
  return themes.value.filter((x) => {
    if (tab.value === "favorites" ? !x.favorite : x.origin !== tab.value) return false;
    if (tag.value && !x.theme.tags.includes(tag.value)) return false;
    if (difficulty.value && x.theme.difficulty !== difficulty.value) return false;
    if (onlyCompatible.value && !x.compatible) return false;
    return !q || `${x.theme.name} ${pick(x.theme.description, locale.value).text} ${x.theme.tags.join(" ")}`.toLowerCase().includes(q);
  });
});

async function run(fn: () => Promise<unknown>, okText: string) {
  busy.value = true;
  op.notice = null;
  try {
    await fn();
    op.notice = { kind: "ok", text: okText };
    version.value++;
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  } finally {
    busy.value = false;
  }
}

async function doImport() {
  const url = importUrl.value.trim();
  if (!url) return;
  await run(async () => {
    await api.importTheme(url);
    importUrl.value = "";
    tab.value = "imported";
  }, t("themes.imported"));
}

async function doSave() {
  const name = saveName.value.trim();
  if (!name) return;
  const id = name.toLowerCase().normalize("NFD").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "my-setup";
  await run(async () => {
    await api.saveTheme(id, name);
    saveName.value = "";
    tab.value = "mine";
  }, t("themes.saved"));
}

const swatch = (x: ThemeView) => x.theme.palette?.colors ?? {};
</script>

<template>
  <h1>{{ t("nav.themes") }}</h1>

  <div class="tabs" role="tablist">
    <button v-for="x in ['official', 'imported', 'mine', 'favorites'] as Tab[]" :key="x" role="tab" :aria-selected="tab === x" :class="{ on: tab === x }" @click="tab = x">
      {{ t("themes.tab." + x) }}
    </button>
  </div>

  <div class="filters">
    <input v-model="query" type="search" :placeholder="t('themes.search')" :aria-label="t('themes.search')" />
    <select v-model="tag" :aria-label="t('themes.tag')">
      <option value="">{{ t("themes.anyTag") }}</option>
      <option v-for="g in tags" :key="g" :value="g">{{ g }}</option>
    </select>
    <select v-model="difficulty" :aria-label="t('themes.difficulty')">
      <option value="">{{ t("themes.anyDifficulty") }}</option>
      <option v-for="d in ['beginner', 'intermediate', 'advanced']" :key="d" :value="d">{{ t("difficulty." + d) }}</option>
    </select>
    <label class="check"><input v-model="onlyCompatible" type="checkbox" /> {{ t("themes.onlyCompatible") }}</label>
  </div>

  <p v-if="!shown.length" class="muted">{{ t("themes.empty." + tab) }}</p>
  <div class="grid">
    <router-link v-for="x in shown" :key="x.theme.id" :to="'/themes/' + x.theme.id" class="card theme">
      <div class="cover" :style="{ background: swatch(x).base ?? 'var(--surface)', color: swatch(x).text ?? 'var(--text)' }">
        <strong>{{ x.theme.name }}</strong>
        <span class="dots">
          <i v-for="(c, k) in swatch(x)" :key="k" :style="{ background: c }" :title="k + ' ' + c" />
        </span>
      </div>
      <p class="muted">{{ pick(x.theme.description, locale).text }}</p>
      <div class="badges">
        <span class="badge">{{ t("difficulty." + (x.theme.difficulty ?? "beginner")) }}</span>
        <span v-if="x.applied" class="badge ok">{{ t("themes.applied") }}</span>
        <span v-if="x.origin === 'imported'" class="badge warn">{{ t("themes.unverified") }}</span>
        <span v-if="!x.compatible" class="badge bad">{{ t("tool.incompatible") }}</span>
        <span v-if="x.favorite" class="badge">★</span>
      </div>
    </router-link>
  </div>

  <section class="card">
    <h2>{{ t("themes.importTitle") }}</h2>
    <p class="muted">{{ t("themes.importHint") }}</p>
    <div class="row">
      <input v-model="importUrl" type="url" class="wide" placeholder="https://github.com/user/repo" :aria-label="t('themes.importTitle')" @keyup.enter="doImport" />
      <button class="btn" :disabled="busy || !importUrl.trim()" @click="doImport">{{ t("themes.import") }}</button>
    </div>
  </section>

  <section class="card">
    <h2>{{ t("themes.saveTitle") }}</h2>
    <p class="muted">{{ t("themes.saveHint") }}</p>
    <div class="row">
      <input v-model="saveName" class="wide" :placeholder="t('themes.saveName')" :aria-label="t('themes.saveName')" @keyup.enter="doSave" />
      <button class="btn" :disabled="busy || !saveName.trim()" @click="doSave">{{ t("themes.save") }}</button>
    </div>
  </section>
</template>

<style scoped>
.tabs { display: flex; gap: 4px; margin-bottom: var(--space); border-bottom: 1px solid var(--border); }
.tabs button { font: inherit; color: var(--text-muted); background: none; border: 0; padding: 8px 14px; cursor: pointer; border-bottom: 2px solid transparent; }
.tabs button.on { color: var(--text); border-bottom-color: var(--accent); }
.filters { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; margin-bottom: var(--space); }
.filters input[type="search"] { font: inherit; padding: 8px 10px; color: var(--text); background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius); }
.check { display: flex; gap: 6px; align-items: center; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space); margin-bottom: var(--space); }
.theme { color: inherit; text-decoration: none; margin: 0; display: flex; flex-direction: column; gap: 6px; }
.theme:hover { border-color: var(--accent); }
.cover { border-radius: var(--radius); padding: 14px; min-height: 90px; display: flex; flex-direction: column; justify-content: space-between; gap: 12px; border: 1px solid var(--border); }
.dots { display: flex; gap: 6px; }
.dots i { width: 18px; height: 18px; border-radius: 50%; border: 1px solid rgba(128, 128, 128, 0.5); display: inline-block; }
.badges { display: flex; gap: 6px; flex-wrap: wrap; margin-top: auto; }
.badge { font-size: 0.8em; padding: 2px 8px; border: 1px solid var(--border); border-radius: 99px; color: var(--text-muted); }
.badge.ok { color: var(--success); border-color: var(--success); }
.badge.warn { color: var(--warning); border-color: var(--warning); }
.badge.bad { color: var(--danger); border-color: var(--danger); }
.wide { flex: 1; font: inherit; padding: 8px 10px; color: var(--text); background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius); }
</style>
