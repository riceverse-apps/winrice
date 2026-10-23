<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { Plan } from "@/api";

const props = defineProps<{ plan: Plan | null; busy: boolean; title: string; canTry: boolean }>();
const emit = defineEmits<{ confirm: [tryMode: boolean]; cancel: [] }>();
const { t } = useI18n();
const dlg = ref<HTMLDialogElement>();

watch(
  () => props.plan,
  (p) => {
    if (p && !dlg.value?.open) dlg.value?.showModal();
    if (!p && dlg.value?.open) dlg.value.close();
  },
);

function describe(step: Plan["steps"][number]): string {
  const files = (step.files as string[] | undefined)?.length ?? 0;
  return t("plan.step." + step.kind, { tool: step.tool ?? "", id: step.id ?? "", name: step.name ?? "", files });
}
</script>

<template>
  <dialog ref="dlg" class="dialog" @cancel.prevent="!busy && emit('cancel')">
    <template v-if="plan">
      <h2>{{ title }}</h2>
      <p class="muted">{{ t("plan.intro") }}</p>
      <ol class="steps">
        <li v-for="(s, i) in plan.steps" :key="i">{{ describe(s) }}</li>
        <li v-if="!plan.steps.length" class="muted">{{ t("plan.empty") }}</li>
      </ol>
      <p class="muted">{{ t("plan.safety") }}</p>
      <div class="actions">
        <button class="btn secondary" :disabled="busy" @click="emit('cancel')">{{ t("common.cancel") }}</button>
        <button v-if="canTry" class="btn secondary" :disabled="busy" @click="emit('confirm', true)">
          {{ t("plan.tryButton") }}
        </button>
        <button class="btn" :disabled="busy" @click="emit('confirm', false)">
          {{ busy ? t("common.working") : t("common.confirm") }}
        </button>
      </div>
    </template>
  </dialog>
</template>

<style scoped>
.dialog {
  background: var(--bg-elevated); color: var(--text); border: 1px solid var(--border);
  border-radius: var(--radius); padding: calc(var(--space) * 1.5); max-width: 520px; width: 90%;
}
.dialog::backdrop { background: rgba(0, 0, 0, 0.5); }
.steps { padding-left: 1.2em; line-height: 1.7; }
.actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>
