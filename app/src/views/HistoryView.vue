<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, type HistoryEntry } from "@/api";
import { errorText, op, version } from "@/composables/operations";

const { t, te, locale } = useI18n();
const action = (a: string) => (te("history.action." + a) ? t("history.action." + a) : a);
const rows = ref<HistoryEntry[]>([]);
const load = async () => (rows.value = await api.listHistory());
onMounted(load);
watch(version, load);
async function undo(r: HistoryEntry) {
  op.busy = true;
  op.notice = null;
  try {
    const out = await api.undoHistory(r.id);
    op.notice = out.ok ? { kind: "ok", text: t("history.undone") } : { kind: "error", text: errorText(out.error ?? "") };
  } catch (e) {
    op.notice = { kind: "error", text: errorText(e) };
  } finally {
    op.busy = false;
    version.value++;
  }
}
const when = (ts: number) => new Date(ts * 1000).toLocaleString(locale.value);
</script>

<template>
  <h1>{{ t("nav.history") }}</h1>
  <p v-if="!rows.length" class="muted">{{ t("history.empty") }}</p>
  <div v-for="r in rows" :key="r.id" class="card">
    <strong>{{ action(r.action) }}</strong> · {{ r.tool === "all" ? t("history.all") : r.tool }}
    <span class="muted"> · {{ when(r.ts) }} · {{ t("history.status." + r.status) }}</span>
    <span v-if="r.undone" class="badge">{{ t("history.wasUndone") }}</span>
    <button v-else-if="r.undoable" class="btn secondary" :disabled="op.busy" @click="undo(r)">{{ t("history.undo") }}</button>
    <p v-if="r.detail" class="muted">{{ r.detail }}</p>
  </div>
</template>

<style scoped>
.badge { font-size: 0.8em; padding: 2px 8px; border: 1px solid var(--border); border-radius: 99px; color: var(--text-muted); margin-left: 8px; }
.btn { margin-left: 8px; padding: 4px 10px; }
</style>
