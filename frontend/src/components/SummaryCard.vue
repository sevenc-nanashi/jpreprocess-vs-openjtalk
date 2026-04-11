<script setup lang="ts">
import type { Stats } from "../types";

const props = defineProps<{
  label: string;
  stats: Stats;
}>();

function pct(n: number): string {
  if (!props.stats.total) return "0%";
  return (n / props.stats.total) * 100 + "%";
}

function formatInteger(n: number): string {
  return Math.round(n).toLocaleString("ja-JP");
}

function formatDurationMs(n: number): string {
  return n.toLocaleString("ja-JP", {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  });
}
</script>

<template>
  <div class="summary-card">
    <div class="summary-title">{{ label }}</div>
    <div class="summary-row">
      <span class="badge badge-match">{{ stats.matches }}</span>
      <span class="summary-label">一致</span>
      <span class="badge badge-light">{{ stats.lightMismatches }}</span>
      <span class="summary-label">軽微な差異</span>
      <span class="badge badge-fatal">{{ stats.fatalMismatches }}</span>
      <span class="summary-label">差異あり</span>
      <span class="badge badge-error">{{ stats.jpErrors + stats.ojtErrors }}</span>
      <span class="summary-label">エラー</span>
    </div>
    <div class="summary-progress">
      <div
        class="progress-match"
        :style="{ width: pct(stats.matches) }"
        :title="`一致: ${stats.matches}`"
      />
      <div
        class="progress-light"
        :style="{ width: pct(stats.lightMismatches) }"
        :title="`軽微な差異: ${stats.lightMismatches}`"
      />
      <div
        class="progress-fatal"
        :style="{ width: pct(stats.fatalMismatches) }"
        :title="`差異あり: ${stats.fatalMismatches}`"
      />
      <div
        class="progress-error"
        :style="{ width: pct(stats.jpErrors + stats.ojtErrors) }"
        :title="`エラー: ${stats.jpErrors + stats.ojtErrors}`"
      />
    </div>
    <div class="summary-total">
      計 {{ formatInteger(stats.total) }} 文 / {{ formatInteger(stats.characters) }} 文字
    </div>
    <div class="summary-throughput">
      {{ formatInteger(stats.throughputCharsPerSecond) }} chars/s /
      {{ formatDurationMs(stats.extractionDurationMs) }} ms
    </div>
  </div>
</template>

<style scoped>
.summary-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 12px 16px;
}
.summary-title {
  font-weight: 600;
  margin-bottom: 8px;
  font-size: 13px;
  color: var(--color-text-muted);
}
.summary-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}
.summary-label {
  font-size: 12px;
  color: var(--color-text-muted);
  margin-right: 8px;
}
.summary-progress {
  display: flex;
  height: 6px;
  border-radius: 3px;
  overflow: hidden;
  background: var(--color-border);
  margin-bottom: 4px;
}
.progress-match { background: var(--color-match); }
.progress-light { background: var(--color-light); }
.progress-fatal { background: var(--color-fatal); }
.progress-error { background: var(--color-error); }
.summary-total {
  font-size: 12px;
  color: var(--color-text-muted);
  text-align: right;
}
.summary-throughput {
  font-size: 12px;
  color: var(--color-text-muted);
  text-align: right;
}
</style>
