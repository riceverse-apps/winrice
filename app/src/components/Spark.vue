<script setup lang="ts">
import { computed } from "vue";

// Last 60 seconds as a tiny line chart. `max` fixes the scale (e.g. 100 for percentages);
// without it the chart scales to the largest value seen.
const props = defineProps<{ values: number[]; max?: number }>();
const W = 120;
const H = 24;
const points = computed(() => {
  const v = props.values;
  if (v.length < 2) return "";
  const top = props.max ?? Math.max(1, ...v);
  return v.map((y, i) => `${((i / 59) * W).toFixed(1)},${(H - (Math.min(y, top) / top) * H).toFixed(1)}`).join(" ");
});
</script>

<template>
  <svg :viewBox="`0 0 ${W} ${H}`" class="spark" preserveAspectRatio="none" aria-hidden="true">
    <polyline :points="points" fill="none" stroke="var(--accent)" stroke-width="1.5" vector-effect="non-scaling-stroke" />
  </svg>
</template>

<style scoped>
.spark { width: 100%; height: 28px; display: block; margin-top: 4px; background: var(--surface); border-radius: 4px; }
</style>
