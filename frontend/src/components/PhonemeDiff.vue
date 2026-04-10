<script setup lang="ts">
import type { Phoneme } from "../types";

defineProps<{
  phonemes: Phoneme[];
}>();
</script>

<template>
  <span class="phoneme-list">
    <span
      v-for="(ph, i) in phonemes"
      :key="i"
      class="phoneme"
      :class="`phoneme-${ph.diff}`"
    >{{ ph.value }}</span>
  </span>
</template>
