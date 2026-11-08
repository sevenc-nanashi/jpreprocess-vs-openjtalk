<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Entry, EntryChunk, FileResult } from "../types";
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

type EntryBlock = { file: string; count: number } & ({ entries: Entry[] } | { chunk: EntryChunk });

const blocks = computed<EntryBlock[]>(() =>
  props.files.flatMap((file): EntryBlock[] => {
    if (props.selectedFile && file.file !== props.selectedFile) return [];
    if (file.chunks) {
      return file.chunks.map((chunk) => ({
        file: file.file,
        chunk,
        count:
          (props.showMatch ? chunk.counts.matches : 0) +
          (props.showLight ? chunk.counts.light : 0) +
          (props.showFatal ? chunk.counts.fatal : 0) +
          (props.showError ? chunk.counts.errors : 0),
      }));
    }
    const entries = file.entries.filter(shouldShow);
    return [{ file: file.file, entries, count: entries.length }];
  }),
);
const filteredCount = computed(() => blocks.value.reduce((total, block) => total + block.count, 0));
const pageEntries = ref<Array<{ entry: Entry; file: string }>>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const reload = ref(0);

watch(
  () => [blocks.value, page.value, reload.value],
  async (_value, _previous, onCleanup) => {
    const controller = new AbortController();
    onCleanup(() => controller.abort());
    pageEntries.value = [];
    error.value = null;
    loading.value = true;
    try {
      let skip = (page.value - 1) * PAGE_SIZE;
      const visible: Array<{ entry: Entry; file: string }> = [];
      for (const block of blocks.value) {
        if (skip >= block.count) {
          skip -= block.count;
          continue;
        }
        let entries: Entry[];
        if ("entries" in block) {
          entries = block.entries;
        } else {
          const response = await fetch(new URL(block.chunk.path, document.baseURI), {
            signal: controller.signal,
          });
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          if (!response.body) throw new Error("結果の本文がありません");
          const json = new Response(response.body.pipeThrough(new DecompressionStream("gzip")));
          const loaded: Entry[] = await json.json();
          entries = loaded.filter(shouldShow);
        }
        visible.push(
          ...entries
            .slice(skip, skip + PAGE_SIZE - visible.length)
            .map((entry) => ({ entry, file: block.file })),
        );
        skip = 0;
        if (visible.length === PAGE_SIZE) break;
      }
      if (!controller.signal.aborted) pageEntries.value = visible;
    } catch (e) {
      if (!controller.signal.aborted) error.value = String(e);
    } finally {
      if (!controller.signal.aborted) loading.value = false;
    }
  },
  { immediate: true },
);

function shouldShow(e: Entry): boolean {
  if (e.kind === "match") return props.showMatch;
  if (e.kind === "light") return props.showLight;
  if (e.kind === "fatal") return props.showFatal;
  return props.showError;
}

const totalPages = computed(() => Math.max(1, Math.ceil(filteredCount.value / PAGE_SIZE)));

function goPage(n: number) {
  page.value = Math.max(1, Math.min(totalPages.value, n));
}

// Reset page when filters change
watch(
  () => [
    props.files,
    props.selectedFile,
    props.showMatch,
    props.showLight,
    props.showFatal,
    props.showError,
  ],
  () => {
    page.value = 1;
  },
);
</script>

<template>
  <div class="entry-list">
    <div class="entry-count">
      {{ filteredCount }} 件中 {{ filteredCount === 0 ? 0 : (page - 1) * PAGE_SIZE + 1 }}〜{{
        Math.min(page * PAGE_SIZE, filteredCount)
      }}
      件を表示
    </div>

    <div v-if="loading" role="status">読み込み中…</div>
    <div v-else-if="error" role="alert">
      結果を読み込めませんでした。{{ error }}
      <button type="button" @click="reload++">再読み込み</button>
    </div>
    <div v-else class="entries">
      <EntryRow
        v-for="(item, i) in pageEntries"
        :key="i"
        :entry="item.entry"
        :file-label="item.file"
      />
      <div v-if="filteredCount === 0" class="empty">該当するエントリがありません</div>
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
