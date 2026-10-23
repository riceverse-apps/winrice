<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type SyncSummary, type SystemInfo } from "@/api";
import WidgetSettings from "@/components/WidgetSettings.vue";
import { loadGlossary, terms } from "@/composables/glossary";
import { pick } from "@/api";
import { errorText, op, startRestore, version } from "@/composables/operations";
import { appearance } from "@/composables/appearance";
import { LOCALES, setLocale } from "@/i18n";

const { t, locale } = useI18n();
const DEFAULT_ACCENT = "#7c6ff0";
const uninstallToo = ref(false);
const syncing = ref(false);
const synced = ref<SyncSummary | null>(null);
async function sync() {
  syncing.value = true;
  synced.value = null;
  try {
    synced.value = await api.syncCatalog();
    version.value++;
  } catch (e) {
    op.notice = { kind: "error", text: t("catalog.failed", { error: errorText(e) }) };
  } finally {
    syncing.value = false;
  }
}
const system = ref<SystemInfo | null>(null);
onMounted(async () => {
  system.value = await api.systemInfo();
  loadGlossary();
});
</script>

<template>
  <h1>{{ t("nav.settings") }}</h1>

  <section class="card">
    <h2>{{ t("settings.language") }}</h2>
    <div class="row">
      <label for="lang">{{ t("settings.language") }}</label>
      <select id="lang" :value="locale" @change="setLocale(($event.target as HTMLSelectElement).value)">
        <option v-for="l in LOCALES" :key="l.code" :value="l.code">{{ l.label }}</option>
      </select>
    </div>
  </section>

  <section class="card">
    <h2>{{ t("settings.appearance") }}</h2>
    <div class="row">
      <label for="mode">{{ t("settings.mode") }}</label>
      <select id="mode" v-model="appearance.mode">
        <option value="system">{{ t("settings.system") }}</option>
        <option value="light">{{ t("settings.light") }}</option>
        <option value="dark">{{ t("settings.dark") }}</option>
      </select>
    </div>
    <p class="muted">{{ t("settings.modeHint") }}</p>
    <div class="row">
      <label for="accent">{{ t("settings.accent") }}</label>
      <span>
        <input id="accent" type="color" :value="appearance.accent ?? DEFAULT_ACCENT"
               @input="appearance.accent = ($event.target as HTMLInputElement).value" />
        <button class="btn secondary" @click="appearance.accent = null">{{ t("settings.accentDefault") }}</button>
      </span>
    </div>
    <div class="row">
      <label for="fromtheme">{{ t("settings.accentFromTheme") }}</label>
      <input id="fromtheme" type="checkbox" v-model="appearance.accentFromTheme" />
    </div>
    <div class="row">
      <label for="density">{{ t("settings.density") }}</label>
      <select id="density" v-model="appearance.density">
        <option value="comfortable">{{ t("settings.comfortable") }}</option>
        <option value="compact">{{ t("settings.compact") }}</option>
      </select>
    </div>
    <div class="row">
      <label for="reduce">{{ t("settings.reduceMotion") }}</label>
      <input id="reduce" type="checkbox" v-model="appearance.reduceMotion" />
    </div>
  </section>

  <WidgetSettings />

  <section class="card">
    <h2>{{ t("catalog.title") }}</h2>
    <p class="muted">{{ t("catalog.text") }}</p>
    <button class="btn" :disabled="syncing" @click="sync">{{ syncing ? t("common.working") : t("catalog.sync") }}</button>
    <p v-if="synced">{{ t("catalog.result", { tools: synced.tools_updated.length, themes: synced.themes_updated.length }) }}</p>
    <ul v-if="synced?.errors.length"><li v-for="e in synced.errors" :key="e" class="muted">{{ e }}</li></ul>
  </section>

  <section class="card">
    <h2>{{ t("glossary.title") }}</h2>
    <p class="muted">{{ t("glossary.text") }}</p>
    <dl>
      <template v-for="g in terms" :key="g.id">
        <dt><strong>{{ pick(g.term, locale).text }}</strong></dt>
        <dd>{{ pick(g.definition, locale).text }}</dd>
      </template>
    </dl>
  </section>

  <section class="card">
    <h2>{{ t("settings.about") }}</h2>
    <p v-if="system">{{ t("compat.yours", { build: system.build, edition: system.edition }) }}</p>
    <p class="muted">{{ t("settings.emergency") }}</p>
  </section>

  <section class="card">
    <h2>{{ t("restore.title") }}</h2>
    <p class="muted">{{ t("restore.text") }}</p>
    <div class="row">
      <label for="uninstall">{{ t("restore.uninstall") }}</label>
      <input id="uninstall" type="checkbox" v-model="uninstallToo" />
    </div>
    <button class="btn" @click="startRestore(uninstallToo)">{{ t("restore.button") }}</button>
  </section>
</template>
