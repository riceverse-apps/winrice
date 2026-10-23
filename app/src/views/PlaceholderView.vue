<script setup lang="ts">
import { useI18n } from "vue-i18n";
defineProps<{ page: string }>();
const { t } = useI18n();
</script>

<template>
  <h1>{{ t("nav." + page) }}</h1>
  <div class="empty muted">{{ t("common.comingSoon") }}</div>
</template>
