<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { ToolView } from "@/api";
import { startAction } from "@/composables/operations";

const props = defineProps<{ tool: ToolView }>();
const { t } = useI18n();
const enabled = computed(() => props.tool.installed && props.tool.toggleable && props.tool.compatible);
const label = computed(() =>
  !props.tool.compatible ? t("tool.incompatible") : !props.tool.installed ? t("tool.needsInstall") : !props.tool.toggleable ? t("tool.noToggle") : props.tool.manifest.name,
);

function toggle() {
  startAction(props.tool.manifest.id, props.tool.active ? "deactivate" : "activate", props.tool.manifest.name);
}
</script>

<template>
  <button
    class="switch"
    role="switch"
    :aria-checked="tool.active"
    :aria-label="t('tool.toggle', { name: tool.manifest.name })"
    :title="label"
    :disabled="!enabled"
    @click.stop="toggle"
  >
    <span class="knob" />
  </button>
</template>

<style scoped>
.switch {
  width: 44px; height: 24px; border-radius: 12px; border: 1px solid var(--border);
  background: var(--surface); position: relative; cursor: pointer; padding: 0; flex: none;
}
.switch[aria-checked="true"] { background: var(--accent); border-color: var(--accent); }
.switch:disabled { opacity: 0.45; cursor: not-allowed; }
.knob {
  position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%;
  background: var(--text-muted); transition: transform 0.15s;
}
.switch[aria-checked="true"] .knob { transform: translateX(20px); background: var(--accent-text); }
</style>
