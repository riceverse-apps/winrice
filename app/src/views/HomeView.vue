<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, type ActiveTheme, type SystemInfo, type ToolView } from "@/api";
import { op, startRestore, undoTheme, version } from "@/composables/operations";

const { t } = useI18n();
const active = ref<ToolView[]>([]);
const system = ref<SystemInfo | null>(null);
const theme = ref<ActiveTheme | null>(null);
const load = async () => {
  active.value = (await api.listTools()).filter((x) => x.active);
  system.value = await api.systemInfo();
  theme.value = await api.activeTheme();
};
async function ack() {
  await api.ackBuild();
  await load();
}
onMounted(load);
watch(version, load);
</script>

<template>
  <h1>{{ t("home.title") }}</h1>
  <p class="muted">{{ t("home.subtitle") }}</p>
  <div v-if="system?.build_changed" class="card warnbox" role="alert">
    <strong>{{ t("home.buildChanged", { from: system.previous_build, to: system.build }) }}</strong>
    <p>{{ t("home.buildChangedHint") }}</p>
    <button class="btn secondary" @click="ack">{{ t("common.dismiss") }}</button>
  </div>
  <div class="card">
    <h2>{{ t("home.status") }}</h2>
    <p v-if="theme">
      {{ t("home.theme") }}: <router-link :to="'/themes/' + theme.id">{{ theme.name }}</router-link>
      <button class="btn secondary" :disabled="op.busy" @click="undoTheme">{{ t("themes.undo") }}</button>
    </p>
    <p v-if="!active.length" class="muted">{{ t("home.noTools") }}</p>
    <ul v-else>
      <li v-for="x in active" :key="x.manifest.id">
        <router-link :to="'/tools/' + x.manifest.id">{{ x.manifest.name }}</router-link>
      </li>
    </ul>
    <router-link class="btn" to="/themes">{{ t("home.exploreThemes") }}</router-link>
    <router-link class="btn secondary" to="/tools">{{ t("home.exploreTools") }}</router-link>
    <button class="btn secondary" @click="startRestore(false)">{{ t("home.restore") }}</button>
  </div>
</template>

<style scoped>
.warnbox { border-color: var(--warning); }
</style>
