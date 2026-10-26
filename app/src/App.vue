<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { onBeforeUnmount, onMounted, watch } from "vue";
import { op, confirm, cancel, startAction, listenForReverts, version } from "@/composables/operations";
import { api } from "@/api";
import { themeAccent } from "@/composables/appearance";
import PlanDialog from "@/components/PlanDialog.vue";
import TrialDialog from "@/components/TrialDialog.vue";
import OnboardingDialog from "@/components/OnboardingDialog.vue";

let unlisten: (() => void) | undefined;
const refreshAccent = async () => (themeAccent.value = (await api.activeTheme())?.accent ?? null);
onMounted(async () => {
  unlisten = await listenForReverts();
  refreshAccent();
});
watch(version, refreshAccent);
onBeforeUnmount(() => unlisten?.());

defineProps<{ pages: readonly string[] }>();
const { t } = useI18n();
const route = useRoute();
</script>

<template>
  <router-view v-if="route.name === 'widget'" />
  <div v-else class="shell">
    <nav class="sidebar" :aria-label="t('nav.label')">
      <div class="brand">
        <img src="/logo-mark.png" alt="" />
        <span>Win<b>Rice</b></span>
      </div>
      <router-link v-for="p in pages" :key="p" class="nav-link" :to="p === 'home' ? '/' : '/' + p">
        {{ t("nav." + p) }}
      </router-link>
    </nav>
    <main class="content">
      <div v-if="op.notice" class="notice" :class="op.notice.kind" :role="op.notice.kind === 'error' ? 'alert' : 'status'">
        {{ op.notice.text }}
        <button class="btn secondary" @click="op.notice = null">{{ t("common.dismiss") }}</button>
      </div>
      <div v-if="op.conflict" class="notice error" role="alert">
        <span>{{ t("conflict.text", { name: op.conflict.name, others: op.conflict.others.join(", ") }) }}</span>
        <span class="btns">
          <button class="btn secondary" @click="op.conflict = null">{{ t("common.cancel") }}</button>
          <button class="btn" @click="startAction(op.conflict.id, 'activate', op.conflict.name, true)">
            {{ t("conflict.resolve", { others: op.conflict.others.join(", ") }) }}
          </button>
        </span>
      </div>
      <router-view />
    </main>
    <PlanDialog :plan="op.plan" :busy="op.busy" :title="op.title" :can-try="op.canTry" @confirm="confirm" @cancel="cancel" />
    <TrialDialog />
    <OnboardingDialog />
  </div>
</template>

<style>
.notice {
  display: flex; justify-content: space-between; align-items: center; gap: 12px;
  padding: 10px 14px; border-radius: var(--radius); margin-bottom: var(--space);
  border: 1px solid var(--border); background: var(--bg-elevated);
}
.notice .btns { display: flex; gap: 8px; }
.notice.ok { border-color: var(--success); }
.notice.error { border-color: var(--danger); }
</style>
