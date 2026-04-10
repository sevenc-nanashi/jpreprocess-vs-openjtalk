<script setup lang="ts">
import { ref, onMounted } from "vue";
import type { Results } from "./types";
import SummaryCard from "./components/SummaryCard.vue";
import EntryList from "./components/EntryList.vue";

const results = ref<Results | null>(null);
const error = ref<string | null>(null);
const loading = ref(true);

const selectedFile = ref<string | null>(null);
const showMatch = ref(false);
const showLight = ref(true);
const showFatal = ref(true);
const showError = ref(true);

onMounted(async () => {
  try {
    const res = await fetch("./results.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    results.value = await res.json();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});

function formatDate(iso: string): string {
  if (!iso) return "不明";
  try {
    return new Date(iso).toLocaleString("ja-JP");
  } catch {
    return iso;
  }
}
</script>

<template>
  <div class="container">
    <header class="app-header">
      <h1>jpreprocess vs OpenJTalk</h1>
      <div v-if="results" class="app-meta">
        <span>コミット:
          <code>{{ results.commit.slice(0, 7) }}</code>
        </span>
        <span>生成日時: {{ formatDate(results.generatedAt) }}</span>
      </div>
    </header>

    <div v-if="loading" class="status">読み込み中…</div>
    <div v-else-if="error" class="status error">
      results.json の読み込みに失敗しました: {{ error }}
    </div>

    <template v-else-if="results">
      <!-- 全体サマリ -->
      <section class="section">
        <SummaryCard label="全体" :stats="results.totals" />
      </section>

      <!-- ファイル別サマリ -->
      <section class="section">
        <h2 class="section-title">ファイル別</h2>
        <div class="file-grid">
          <SummaryCard
            v-for="f in results.files"
            :key="f.file"
            :label="f.file"
            :stats="f.stats"
          />
        </div>
      </section>

      <!-- フィルタ + エントリ一覧 -->
      <section class="section">
        <h2 class="section-title">差異一覧</h2>
        <div class="filter-bar">
          <select v-model="selectedFile" class="file-select">
            <option :value="null">全ファイル</option>
            <option v-for="f in results.files" :key="f.file" :value="f.file">
              {{ f.file }}
            </option>
          </select>
          <label class="filter-check">
            <input type="checkbox" v-model="showMatch" />
            <span class="badge badge-match">一致</span>
          </label>
          <label class="filter-check">
            <input type="checkbox" v-model="showLight" />
            <span class="badge badge-light">軽微な差異</span>
          </label>
          <label class="filter-check">
            <input type="checkbox" v-model="showFatal" />
            <span class="badge badge-fatal">差異あり</span>
          </label>
          <label class="filter-check">
            <input type="checkbox" v-model="showError" />
            <span class="badge badge-error">エラー</span>
          </label>
        </div>
        <EntryList
          :files="results.files"
          :selected-file="selectedFile"
          :show-match="showMatch"
          :show-light="showLight"
          :show-fatal="showFatal"
          :show-error="showError"
        />
      </section>
    </template>
  </div>
</template>

<style scoped>
.app-header {
  padding: 24px 0 16px;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 24px;
}
.app-header h1 {
  font-size: 22px;
  margin-bottom: 4px;
}
.app-meta {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--color-text-muted);
}
.app-meta code {
  font-family: monospace;
  color: var(--color-text);
}
.section {
  margin-bottom: 32px;
}
.section-title {
  font-size: 16px;
  margin-bottom: 12px;
  color: var(--color-text-muted);
}
.file-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 10px;
}
.filter-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}
.file-select {
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 13px;
}
.filter-check {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
.status {
  padding: 32px;
  text-align: center;
  color: var(--color-text-muted);
}
.status.error {
  color: var(--color-fatal);
}
</style>
