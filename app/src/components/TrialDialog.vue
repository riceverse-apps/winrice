<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { op, keepTrial, revertTrial } from "@/composables/operations";

const { t } = useI18n();
const dlg = ref<HTMLDialogElement>();
const left = ref(0);
let timer: number | undefined;

function tick() {
  left.value = op.trial ? Math.max(0, Math.ceil((op.trial.deadline - Date.now()) / 1000)) : 0;
}

watch(
  () => op.trial,
  (tr) => {
    window.clearInterval(timer);
    if (tr) {
      tick();
      timer = window.setInterval(tick, 250);
      if (!dlg.value?.open) dlg.value?.showModal();
    } else if (dlg.value?.open) dlg.value.close();
  },
);
onBeforeUnmount(() => window.clearInterval(timer));
</script>

<template>
  <dialog ref="dlg" class="trial" aria-live="assertive" @cancel.prevent>
    <template v-if="op.trial">
      <h2>{{ t("trial.title", { name: op.trial.name }) }}</h2>
      <p class="count">{{ t("trial.countdown", { s: left }) }}</p>
      <p class="muted">{{ t("trial.hint") }}</p>
      <div class="actions">
        <button class="btn secondary" @click="revertTrial">{{ t("trial.revert") }}</button>
        <button class="btn" @click="keepTrial">{{ t("trial.keep") }}</button>
      </div>
    </template>
  </dialog>
</template>

<style scoped>
.trial {
  background: var(--bg-elevated); color: var(--text); border: 2px solid var(--accent);
  border-radius: var(--radius); padding: calc(var(--space) * 1.5); max-width: 460px; width: 90%;
}
.trial::backdrop { background: rgba(0, 0, 0, 0.6); }
.count { font-size: 1.4em; font-weight: 600; }
.actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>
