<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, pick, type ToolView } from "@/api";
import { version } from "@/composables/operations";
import ToolToggle from "@/components/ToolToggle.vue";

const { t, locale } = useI18n();
const tools = ref<ToolView[]>([]);
const query = ref("");
const loadError = ref("");
const onlyFav = ref(false);
async function star(tool: ToolView) {
  await api.toggleToolFavorite(tool.manifest.id);
  version.value++;
}

async function load() {
  try {
    tools.value = await api.listTools();
  } catch (e) {
    loadError.value = String(e);
  }
}
onMounted(load);
watch(version, load);

const groups = computed(() => {
  const q = query.value.trim().toLowerCase();
  const map = new Map<string, ToolView[]>();
  for (const tool of tools.value) {
    const m = tool.manifest;
    if (onlyFav.value && !tool.favorite) continue;
    if (q && !`${m.name} ${pick(m.summary, locale.value).text}`.toLowerCase().includes(q)) continue;
    map.set(m.category, [...(map.get(m.category) ?? []), tool]);
  }
  return [...map.entries()];
});

function state(tool: ToolView): string {
  if (!tool.installed) return "notInstalled";
  if (!tool.toggleable) return "installed";
  return tool.active ? "active" : "inactive";
}
</script>

<template>
  <h1>{{ t("nav.tools") }}</h1>
  <input v-model="query" class="search" type="search" :placeholder="t('tool.search')" :aria-label="t('tool.search')" />
  <label class="favfilter"><input v-model="onlyFav" type="checkbox" /> {{ t("tool.onlyFavorites") }}</label>
  <p v-if="loadError" class="muted" role="alert">{{ loadError }}</p>

  <section v-for="[cat, list] in groups" :key="cat" class="group">
    <h2>{{ t("category." + cat) }}</h2>
    <div class="grid">
      <router-link v-for="tool in list" :key="tool.manifest.id" :to="'/tools/' + tool.manifest.id" class="card tool">
        <div class="head">
          <strong>
            <button class="star" :aria-pressed="tool.favorite" :aria-label="t('themes.favorite')" @click.prevent.stop="star(tool)">{{ tool.favorite ? "★" : "☆" }}</button>
            {{ tool.manifest.name }}
          </strong>
          <ToolToggle :tool="tool" />
        </div>
        <p class="muted">{{ pick(tool.manifest.summary, locale).text }}</p>
        <div class="badges">
          <span class="badge">{{ t("difficulty." + tool.manifest.difficulty) }}</span>
          <span class="badge" :class="state(tool)">{{ t("state." + state(tool)) }}</span>
          <span v-if="tool.manifest.requires.admin" class="badge">{{ t("tool.admin") }}</span>
          <span v-if="tool.manifest.paid" class="badge warn">{{ t("tool.paid") }}</span>
          <span v-if="tool.manifest.restricted_license" class="badge warn">{{ t("tool.restricted") }}</span>
          <span v-if="!tool.compatible" class="badge bad">{{ t("tool.incompatible") }}</span>
        </div>
      </router-link>
    </div>
  </section>
</template>

<style scoped>
.search {
  font: inherit; width: 100%; max-width: 360px; margin-bottom: var(--space); padding: 8px 10px;
  color: var(--text); background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius);
}
.star { background: none; border: 0; cursor: pointer; color: var(--accent); font-size: 1.1em; padding: 0 4px 0 0; }
.favfilter { display: inline-flex; gap: 6px; align-items: center; margin-left: 12px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space); }
.tool { color: inherit; text-decoration: none; margin: 0; display: flex; flex-direction: column; gap: 4px; }
.tool:hover { border-color: var(--accent); }
.head { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.group { margin-bottom: calc(var(--space) * 1.5); }
.badges { display: flex; gap: 6px; flex-wrap: wrap; margin-top: auto; }
.badge { font-size: 0.8em; padding: 2px 8px; border: 1px solid var(--border); border-radius: 99px; color: var(--text-muted); }
.badge.warn { color: var(--warning); border-color: var(--warning); }
.badge.bad { color: var(--danger); border-color: var(--danger); }
.badge.active { color: var(--success); border-color: var(--success); }
</style>
