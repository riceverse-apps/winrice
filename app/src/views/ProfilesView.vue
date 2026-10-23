<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, type Profile, type ToolView } from "@/api";
import { errorText, op, version } from "@/composables/operations";

const { t } = useI18n();
const profiles = ref<Profile[]>([]);
const tools = ref<ToolView[]>([]);
const paused = ref<string[] | null>(null);
const name = ref("");

async function load() {
  [profiles.value, tools.value, paused.value] = await Promise.all([api.listProfiles(), api.listTools(), api.pausedState()]);
}
onMounted(load);
watch(version, load);

const nameOf = (id: string) => tools.value.find((x) => x.manifest.id === id)?.manifest.name ?? id;
const activeNow = () => tools.value.filter((x) => x.active).map((x) => x.manifest.name);

async function run(fn: () => Promise<{ outcome: { ok: boolean; error: string | null }; skipped: string[] }>) {
  op.busy = true;
  op.notice = null;
  try {
    const r = await fn();
    op.notice = r.outcome.ok
      ? { kind: "ok", text: r.skipped.length ? t("widget.doneSkipped", { names: r.skipped.join(", ") }) : t("widget.done") }
      : { kind: "error", text: t("plan.failedRolledBack", { error: errorText(r.outcome.error ?? "") }) };
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  } finally {
    op.busy = false;
    version.value++;
  }
}

async function save() {
  const n = name.value.trim();
  if (!n) return;
  try {
    await api.saveProfile(n);
    name.value = "";
    op.notice = { kind: "ok", text: t("profiles.saved") };
    version.value++;
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  }
}
</script>

<template>
  <h1>{{ t("nav.profiles") }}</h1>
  <p class="muted">{{ t("profiles.text") }}</p>

  <section class="card">
    <h2>{{ paused ? t("profiles.pausedTitle") : t("profiles.pauseTitle") }}</h2>
    <p class="muted">{{ paused ? t("profiles.pausedText", { names: paused.map(nameOf).join(", ") }) : t("profiles.pauseText") }}</p>
    <button v-if="!paused" class="btn secondary" :disabled="op.busy" @click="run(api.pause)">⏸ {{ t("widget.pause") }}</button>
    <button v-else class="btn" :disabled="op.busy" @click="run(api.resume)">▶ {{ t("widget.resume") }}</button>
  </section>

  <p v-if="!profiles.length" class="muted">{{ t("profiles.empty") }}</p>
  <div v-for="p in profiles" :key="p.name" class="card">
    <div class="row">
      <div>
        <strong>{{ p.name }}</strong>
        <p class="muted">{{ p.tools.length ? p.tools.map(nameOf).join(", ") : t("profiles.allOff") }}</p>
      </div>
      <span class="btns">
        <button class="btn" :disabled="op.busy" @click="run(() => api.applyProfile(p.name))">{{ t("widget.apply") }}</button>
        <button class="btn secondary" :disabled="op.busy" @click="api.deleteProfile(p.name).then(() => version++)">{{ t("themes.delete") }}</button>
      </span>
    </div>
  </div>

  <section class="card">
    <h2>{{ t("profiles.newTitle") }}</h2>
    <p class="muted">{{ t("profiles.newText", { names: activeNow().join(", ") || t("profiles.allOff") }) }}</p>
    <div class="row">
      <input v-model="name" class="wide" :placeholder="t('profiles.name')" :aria-label="t('profiles.name')" @keyup.enter="save" />
      <button class="btn" :disabled="!name.trim()" @click="save">{{ t("profiles.save") }}</button>
    </div>
  </section>
</template>

<style scoped>
.btns { display: flex; gap: 8px; }
.wide { flex: 1; font: inherit; padding: 8px 10px; color: var(--text); background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius); }
</style>
