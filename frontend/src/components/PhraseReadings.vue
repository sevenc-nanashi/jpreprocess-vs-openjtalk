<script setup lang="ts">
import type { Phrase } from "../types";

defineProps<{ phrases: Phrase[] }>();
</script>

<template>
  <div class="phrase-readings" aria-label="アクセント句ごとの表記と読み">
    <ruby
      v-for="(phrase, index) in phrases"
      :key="index"
      :class="{
        'reading-light': phrase.diff === 'light',
        'reading-fatal': phrase.diff === 'fatal',
      }"
      :title="
        phrase.diff === 'fatal' ? '読み違い' : phrase.diff === 'light' ? '無声化の違い' : undefined
      "
    >
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
  color: inherit;
}
.reading-light {
  color: var(--color-light);
  background: rgba(250, 204, 21, 0.12);
}
.reading-fatal {
  color: var(--color-fatal);
  background: rgba(248, 113, 113, 0.12);
}
</style>
