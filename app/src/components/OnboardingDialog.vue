<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";

// First run: three short screens (see 05-telas-e-ux.md).
const KEY = "winrice.onboarded";
const { t } = useI18n();
const router = useRouter();
const dlg = ref<HTMLDialogElement>();
const step = ref(0);
const STEPS = ["toggle", "try", "restore"] as const;

onMounted(() => {
  let seen = false;
  try {
    seen = localStorage.getItem(KEY) === "1";
  } catch {
    /* storage unavailable: show it once per run */
  }
  if (!seen) dlg.value?.showModal();
});

function finish(goThemes: boolean) {
  try {
    localStorage.setItem(KEY, "1");
  } catch {
    /* ignore */
  }
  dlg.value?.close();
  if (goThemes) router.push("/themes");
}
</script>

<template>
  <dialog ref="dlg" class="onboarding" aria-labelledby="ob-title" @cancel.prevent="finish(false)">
    <p class="muted dots" aria-hidden="true">
      <i v-for="(s, i) in STEPS" :key="s" :class="{ on: i === step }" />
    </p>
    <h2 id="ob-title">{{ t("onboarding." + STEPS[step] + ".title") }}</h2>
    <p>{{ t("onboarding." + STEPS[step] + ".text") }}</p>
    <div class="actions">
      <button class="btn secondary" @click="finish(false)">{{ t("onboarding.skip") }}</button>
      <button v-if="step > 0" class="btn secondary" @click="step--">{{ t("onboarding.back") }}</button>
      <button v-if="step < STEPS.length - 1" class="btn" @click="step++">{{ t("onboarding.next") }}</button>
      <template v-else>
        <button class="btn secondary" @click="finish(false)">{{ t("onboarding.done") }}</button>
        <button class="btn" @click="finish(true)">{{ t("onboarding.themes") }}</button>
      </template>
    </div>
  </dialog>
</template>

<style scoped>
.onboarding { background: var(--bg-elevated); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); padding: calc(var(--space) * 1.5); max-width: 480px; width: 90%; }
.onboarding::backdrop { background: rgba(0, 0, 0, 0.55); }
.dots { display: flex; gap: 6px; margin: 0 0 8px; }
.dots i { width: 8px; height: 8px; border-radius: 50%; background: var(--border); display: inline-block; }
.dots i.on { background: var(--accent); }
.actions { display: flex; justify-content: flex-end; gap: 8px; flex-wrap: wrap; }
</style>
