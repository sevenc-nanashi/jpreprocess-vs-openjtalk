<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Entry, FileResult } from "../types";
import EntryRow from "./EntryRow.vue";

const props = defineProps<{
  files: FileResult[];
  selectedFile: string | null;
  showMatch: boolean;
  showLight: boolean;
  showFatal: boolean;
  showError: boolean;
}>();

const PAGE_SIZE = 50;
const page = ref(1);

const filteredEntries = computed<Array<{ entry: Entry; file: string }>>(() => {
  const result: Array<{ entry: Entry; file: string }> = [];
  for (const f of props.files) {
    if (props.selectedFile && f.file !== props.selectedFile) continue;
    for (const e of f.entries) {
      if (!shouldShow(e)) continue;
      result.push({ entry: e, file: f.file });
    }
  }
  return result;
});

function shouldShow(e: Entry): boolean {
  if (e.kind === "match") return props.showMatch;
  if (e.kind === "light") return props.showLight;
  if (e.kind === "fatal") return props.showFatal;
  return props.showError;
}

const totalPages = computed(() =>
  Math.max(1, Math.ceil(filteredEntries.value.length / PAGE_SIZE))
);

const pageEntries = computed(() => {
  const start = (page.value - 1) * PAGE_SIZE;
  return filteredEntries.value.slice(start, start + PAGE_SIZE);
});

function goPage(n: number) {
  page.value = Math.max(1, Math.min(totalPages.value, n));
}

// Reset page when filters change
watch(
  () => [props.selectedFile, props.showMatch, props.showLight, props.showFatal, props.showError],
  () => { page.value = 1; }
);
</script>

<template>
  <div class="entry-list">
    <div class="entry-count">
      {{ filteredEntries.length }} 件中 {{ (page - 1) * PAGE_SIZE + 1 }}〜{{ Math.min(page * PAGE_SIZE, filteredEntries.length) }} 件を表示
    </div>

    <div class="entries">
      <EntryRow
        v-for="(item, i) in pageEntries"
        :key="i"
        :entry="item.entry"
        :file-label="item.file"
      />
      <div v-if="filteredEntries.length === 0" class="empty">
        該当するエントリがありません
      </div>
    </div>

    <div v-if="totalPages > 1" class="pagination">
      <button :disabled="page === 1" @click="goPage(1)">«</button>
      <button :disabled="page === 1" @click="goPage(page - 1)">‹</button>
      <span class="page-info">{{ page }} / {{ totalPages }}</span>
      <button :disabled="page === totalPages" @click="goPage(page + 1)">›</button>
      <button :disabled="page === totalPages" @click="goPage(totalPages)">»</button>
    </div>
  </div>
</template>

<style scoped>
.entry-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.entry-count {
  font-size: 12px;
  color: var(--color-text-muted);
}
.entries {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.empty {
  text-align: center;
  color: var(--color-text-muted);
  padding: 32px;
}
.pagination {
  display: flex;
  align-items: center;
  gap: 4px;
  justify-content: center;
}
.pagination button {
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text);
  cursor: pointer;
  font-size: 14px;
}
.pagination button:disabled {
  opacity: 0.4;
  cursor: default;
}
.pagination button:not(:disabled):hover {
  background: var(--color-surface2);
}
.page-info {
  padding: 0 12px;
  font-size: 13px;
  color: var(--color-text-muted);
}
</style>
