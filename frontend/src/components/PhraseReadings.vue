<script setup lang="ts">
import type { Phrase } from "../types";

defineProps<{ phrases: Phrase[] }>();
</script>

<template>
  <div class="phrase-readings" aria-label="アクセント句ごとの表記と読み">
    <ruby v-for="(phrase, index) in phrases" :key="index">
      {{ phrase.text }}<rp>（</rp><rt>{{ phrase.reading }}</rt
      ><rp>）</rp>
    </ruby>
  </div>
</template>

<style scoped>
.phrase-readings {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 16px;
  padding-top: 12px;
  line-height: 1.8;
  overflow-wrap: anywhere;
}
ruby {
  padding: 0 4px;
  border-bottom: 1px solid var(--color-border);
}
rt {
  font-size: 12px;
  color: var(--color-text);
}
</style>
