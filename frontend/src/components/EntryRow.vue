<script setup lang="ts">
import type { Entry } from "../types";
import PhonemeDiff from "./PhonemeDiff.vue";

defineProps<{
  entry: Entry;
  fileLabel: string;
}>();

function kindClass(kind: string): string {
  if (kind === "match") return "match";
  if (kind === "light") return "light";
  if (kind === "fatal") return "fatal";
  return "error";
}

const kindLabel: Record<string, string> = {
  match: "一致",
  light: "軽微な差異",
  fatal: "差異あり",
  jp_error: "JP エラー",
  ojt_error: "OJT エラー",
  both_error: "両エラー",
  jp_panic: "JP パニック",
};
</script>

<template>
  <div class="entry-row" :class="`entry-${entry.kind}`">
    <div class="entry-header">
      <span class="badge" :class="`badge-${kindClass(entry.kind)}`">
        {{ kindLabel[entry.kind] }}
      </span>
      <span class="entry-meta">{{ fileLabel }} #{{ entry.index + 1 }}</span>
      <span v-if="entry.kind === 'fatal' && (entry as any).lengthMismatch" class="entry-length-mismatch">
        (長さ不一致: OJT {{ (entry as any).openjtalk.length }} / JP {{ (entry as any).jpreprocess.length }})
      </span>
    </div>
    <div class="entry-original">{{ entry.original }}</div>
    <template v-if="entry.kind === 'match' || entry.kind === 'light' || entry.kind === 'fatal'">
      <div class="entry-phonemes">
        <span class="entry-label">OJT</span>
        <PhonemeDiff :phonemes="(entry as any).openjtalk" />
      </div>
      <div class="entry-phonemes">
        <span class="entry-label">JP</span>
        <PhonemeDiff :phonemes="(entry as any).jpreprocess" />
      </div>
    </template>
    <template v-else>
      <div v-if="(entry as any).openjtalkError" class="entry-error">
        <span class="entry-label">OJT</span>
        <code>{{ (entry as any).openjtalkError }}</code>
      </div>
      <div v-if="(entry as any).jpreprocessError" class="entry-error">
        <span class="entry-label">JP</span>
        <code>{{ (entry as any).jpreprocessError }}</code>
      </div>
    </template>
  </div>
</template>

<style scoped>
.entry-row {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 10px 12px;
  background: var(--color-surface);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.entry-match   { border-left: 3px solid var(--color-match); }
.entry-light   { border-left: 3px solid var(--color-light); }
.entry-fatal   { border-left: 3px solid var(--color-fatal); }
.entry-jp_error,
.entry-ojt_error,
.entry-both_error,
.entry-jp_panic { border-left: 3px solid var(--color-error); }

.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.entry-meta {
  font-size: 12px;
  color: var(--color-text-muted);
}
.entry-length-mismatch {
  font-size: 11px;
  color: var(--color-text-muted);
}
.entry-original {
  font-size: 14px;
  color: var(--color-text);
}
.entry-phonemes,
.entry-error {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.entry-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-muted);
  width: 28px;
  flex-shrink: 0;
  padding-top: 2px;
}
.entry-error code {
  font-size: 12px;
  color: var(--color-error);
  word-break: break-all;
}
</style>
