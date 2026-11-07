<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type OptimizeSuggestion, type TweakStatus } from "@/api";
import { errorText, op, version } from "@/composables/operations";

const { t } = useI18n();
const tweaks = ref<TweakStatus[]>([]);
const suggestions = ref<OptimizeSuggestion[]>([]);
const picked = ref<Set<string>>(new Set());
const restartNote = ref(false);

async function load() {
  tweaks.value = await api.optimizeStatus();
  suggestions.value = await api.optimizeSuggestions();
  // Start with everything recommended that is not applied yet.
  picked.value = new Set(tweaks.value.filter((x) => x.recommended && !x.optimized).map((x) => x.id));
}
onMounted(load);

const todo = computed(() => tweaks.value.filter((x) => picked.value.has(x.id) && !x.optimized));
const restorable = computed(() => tweaks.value.filter((x) => x.restorable));

function toggle(id: string) {
  const s = new Set(picked.value);
  s.has(id) ? s.delete(id) : s.add(id);
  picked.value = s;
}

async function run(fn: () => Promise<TweakStatus[]>, okText: string, needsRestart: boolean) {
  op.busy = true;
  op.notice = null;
  try {
    tweaks.value = await fn();
    restartNote.value = needsRestart;
    op.notice = { kind: "ok", text: okText };
    version.value++;
    picked.value = new Set(tweaks.value.filter((x) => x.recommended && !x.optimized).map((x) => x.id));
  } catch (e) {
    op.notice = { kind: "error", text: t("optimize.failed", { error: errorText(e) }) };
  } finally {
    op.busy = false;
  }
}

const apply = () => run(() => api.optimizeApply(todo.value.map((x) => x.id)), t("optimize.applied", { n: todo.value.length }), todo.value.some((x) => x.restart));
const restore = (ids: string[]) => run(() => api.optimizeRestore(ids), t("optimize.restored"), false);
const selectRecommended = () => (picked.value = new Set(tweaks.value.filter((x) => x.recommended && !x.optimized).map((x) => x.id)));
</script>

<template>
  <h1>{{ t("nav.optimize") }}</h1>
  <p class="muted">{{ t("optimize.text") }}</p>

  <div v-if="restartNote" class="card note" role="status">{{ t("optimize.restartNote") }}</div>

  <section class="card">
    <h2>{{ t("optimize.suggestionsTitle") }}</h2>
    <p class="muted">{{ t("optimize.suggestionsText") }}</p>
    <ul class="sugg">
      <li v-for="s in suggestions" :key="s.id" :class="s.level">
        <span class="lvl">{{ t("optimize.level." + s.level) }}</span>
        <span>
          <strong>{{ t("optimize.suggestion." + s.id + ".title") }}</strong>
          <span class="muted"> — {{ t("optimize.suggestion." + s.id + ".text") }}</span>
        </span>
      </li>
    </ul>
  </section>

  <section class="card">
    <div class="row head">
      <div>
        <h2>{{ t("optimize.tweaksTitle") }}</h2>
        <p class="muted">{{ t("optimize.tweaksText") }}</p>
      </div>
      <span class="btns">
        <button class="btn secondary" :disabled="op.busy" @click="selectRecommended">{{ t("optimize.selectRecommended") }}</button>
        <button class="btn" :disabled="op.busy || !todo.length" @click="apply">{{ t("optimize.apply", { n: todo.length }) }}</button>
      </span>
    </div>
    <ul class="tweaks">
      <li v-for="x in tweaks" :key="x.id">
        <label>
          <input type="checkbox" :checked="x.optimized || picked.has(x.id)" :disabled="x.optimized || op.busy" @change="toggle(x.id)" />
          <span class="info">
            <strong>{{ t("optimize.tweak." + x.id + ".title") }}</strong>
            <span class="muted">{{ t("optimize.tweak." + x.id + ".text") }}</span>
            <span class="tags">
              <span class="tag">{{ t("optimize.impact." + x.impact) }}</span>
              <span v-if="x.restart" class="tag">{{ t("optimize.needsSignOut") }}</span>
              <span v-if="!x.recommended" class="tag">{{ t("optimize.optional") }}</span>
              <span v-if="x.optimized" class="tag done">{{ t("optimize.on") }}</span>
            </span>
          </span>
        </label>
        <button v-if="x.restorable" class="btn secondary" :disabled="op.busy" @click="restore([x.id])">{{ t("optimize.restore") }}</button>
      </li>
    </ul>
    <button v-if="restorable.length" class="btn secondary" :disabled="op.busy" @click="restore([])">{{ t("optimize.restoreAll") }}</button>
  </section>
</template>

<style scoped>
.head { align-items: flex-start; }
.btns { display: flex; gap: 8px; flex-wrap: wrap; }
.note { border-color: var(--warning); }
ul { list-style: none; padding: 0; margin: 8px 0; }
.sugg li { display: flex; gap: 10px; padding: 6px 0; align-items: baseline; }
.lvl { font-size: 0.75rem; padding: 2px 8px; border-radius: 999px; border: 1px solid var(--border); white-space: nowrap; }
.sugg li.high .lvl { border-color: var(--danger); }
.sugg li.medium .lvl { border-color: var(--warning); }
.tweaks li { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; padding: 10px 0; border-top: 1px solid var(--border); }
.tweaks label { display: flex; gap: 10px; align-items: flex-start; flex: 1; cursor: pointer; }
.info { display: flex; flex-direction: column; gap: 2px; }
.tags { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 4px; }
.tag { font-size: 0.72rem; padding: 1px 8px; border-radius: 999px; border: 1px solid var(--border); }
.tag.done { border-color: var(--success); }
</style>
