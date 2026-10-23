<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, type Issue, type ThemeView } from "@/api";
import { errorText, op, version } from "@/composables/operations";

const props = defineProps<{ view: ThemeView }>();
const { t, locale } = useI18n();

const REPO = "https://github.com/riceverse-apps/winrice";
const LICENSES = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC", "MPL-2.0", "GPL-3.0", "LGPL-3.0", "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "Unlicense"];

const open = ref(false);
const issues = ref<Issue[]>([]);
const folder = ref("");
const cover = ref("");
const form = ref({ name: "", license: "MIT", github: "", description_en: "", description_pt: "" });

function fill() {
  const th = props.view.theme;
  form.value = {
    name: th.name,
    license: LICENSES.includes(th.license) ? th.license : "MIT",
    github: th.author.github ?? "",
    description_en: th.description.en ?? "",
    description_pt: th.description["pt-BR"] ?? "",
  };
}

async function check() {
  issues.value = await api.submissionCheck(props.view.theme.id);
}
async function start() {
  fill();
  folder.value = "";
  open.value = true;
  await check();
}
watch(() => props.view.theme.id, () => (open.value = false));

const errors = computed(() => issues.value.filter((i) => i.level === "error"));
const warnings = computed(() => issues.value.filter((i) => i.level === "warning"));
const ready = computed(() => !errors.value.length);

async function guard(fn: () => Promise<unknown>) {
  op.notice = null;
  try {
    await fn();
    version.value++;
    await check();
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  }
}
const saveMeta = () => guard(() => api.editThemeMeta(props.view.theme.id, { ...form.value }));
const saveCover = () => guard(async () => {
  await api.setThemeCover(props.view.theme.id, cover.value);
  cover.value = "";
});
const prepare = () => guard(async () => (folder.value = await api.prepareSubmission(props.view.theme.id)));
const go = (path: string) => api.openUrl(REPO + path).catch((e) => (op.notice = { kind: "error", text: errorText(e) }));
void locale;
</script>

<template>
  <section class="card">
    <h2>{{ t("submit.title") }}</h2>
    <p class="muted">{{ t("submit.text") }}</p>
    <button v-if="!open" class="btn" @click="start">{{ t("submit.start") }}</button>

    <template v-else>
      <h3>{{ t("submit.step1") }}</h3>
      <ul class="issues">
        <li v-for="i in issues" :key="i.code" :class="i.level">
          {{ i.level === "error" ? "✕" : "⚠" }} {{ t("submit.issue." + i.code) }}
        </li>
        <li v-if="!issues.length" class="ok">✓ {{ t("submit.allGood") }}</li>
      </ul>

      <h3>{{ t("submit.step2") }}</h3>
      <div class="grid">
        <label>{{ t("submit.name") }}<input v-model="form.name" /></label>
        <label>{{ t("submit.github") }}<input v-model="form.github" placeholder="username" /></label>
        <label>
          {{ t("submit.license") }}
          <select v-model="form.license"><option v-for="l in LICENSES" :key="l" :value="l">{{ l }}</option></select>
        </label>
        <label class="wide">{{ t("submit.descEn") }}<input v-model="form.description_en" /></label>
        <label class="wide">{{ t("submit.descPt") }}<input v-model="form.description_pt" /></label>
      </div>
      <button class="btn secondary" @click="saveMeta">{{ t("submit.saveDetails") }}</button>

      <h3>{{ t("submit.step3") }}</h3>
      <p class="muted">{{ t("submit.coverHint") }}</p>
      <div class="row">
        <input v-model="cover" class="wideinput" placeholder="C:\Users\...\screenshot.png" :aria-label="t('submit.cover')" />
        <button class="btn secondary" :disabled="!cover.trim()" @click="saveCover">{{ t("submit.addCover") }}</button>
      </div>

      <h3>{{ t("submit.step4") }}</h3>
      <ul v-if="warnings.length" class="issues">
        <li v-for="i in warnings" :key="i.code" class="warning">⚠ {{ t("submit.issue." + i.code) }}</li>
      </ul>
      <button class="btn" :disabled="!ready" @click="prepare">{{ t("submit.prepare") }}</button>

      <div v-if="folder" class="done">
        <p>{{ t("submit.prepared", { folder }) }}</p>
        <ol>
          <li>{{ t("submit.how1") }}</li>
          <li>{{ t("submit.how2") }}</li>
          <li>{{ t("submit.how3") }}</li>
          <li>{{ t("submit.how4") }}</li>
        </ol>
        <button class="btn secondary" @click="go('')">{{ t("submit.openRepo") }}</button>
        <button class="btn secondary" @click="go('/blob/main/CONTRIBUTING.md')">{{ t("submit.openContributing") }}</button>
      </div>
    </template>
  </section>
</template>

<style scoped>
h3 { margin: 16px 0 6px; font-size: 1em; }
.issues { list-style: none; padding: 0; margin: 0 0 8px; }
.issues li { padding: 2px 0; }
.issues .error { color: var(--danger); }
.issues .warning { color: var(--warning); }
.issues .ok { color: var(--success); }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 10px; }
.grid label { display: flex; flex-direction: column; gap: 4px; }
.grid .wide { grid-column: 1 / -1; }
input { font: inherit; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 8px; }
.wideinput { flex: 1; }
.done { margin-top: 12px; padding: 10px; border: 1px solid var(--success); border-radius: var(--radius); }
.done .btn { margin-right: 8px; }
</style>
