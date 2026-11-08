<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import type { Results } from "../types";
import type { ComparisonResponse } from "../compare.worker";
import SummaryCard from "./SummaryCard.vue";
import EntryList from "./EntryList.vue";

const text = ref("こんにちは、世界。");
const results = ref<Results | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);
const showMatch = ref(true);
const showLight = ref(true);
const showFatal = ref(true);
const showError = ref(true);
let worker: Worker | null = null;

function stop() {
  worker?.terminate();
  worker = null;
  busy.value = false;
}

function compare() {
  stop();
  results.value = null;
  error.value = null;
  busy.value = true;
  try {
    worker = new Worker(new URL("../compare.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = ({ data }: MessageEvent<ComparisonResponse>) => {
      if ("error" in data) error.value = data.error;
      else if (data.results.totals.total === 0) error.value = "比較できる文章を入力してください。";
      else results.value = data.results;
      stop();
    };
    worker.onerror = (event) => {
      error.value = event.message;
      stop();
    };
    worker.postMessage({
      text: text.value,
      baseUrl: new URL(`${import.meta.env.BASE_URL}wasm/`, document.baseURI).href,
    });
  } catch (e) {
    error.value = String(e);
    stop();
  }
}

onBeforeUnmount(stop);
</script>

<template>
  <section class="text-comparison" aria-labelledby="text-comparison-title">
    <h2 id="text-comparison-title">文章を比較</h2>
    <form @submit.prevent="compare">
      <label for="comparison-text">比較する文章</label>
      <textarea id="comparison-text" v-model="text" rows="4" required :disabled="busy"></textarea>
      <div class="actions">
        <button type="submit" :disabled="busy || !text.trim()">比較する</button>
        <button v-if="busy" type="button" @click="stop">キャンセル</button>
        <span role="status">{{ busy ? "辞書を読み込み・比較中…" : "" }}</span>
      </div>
    </form>
    <p v-if="error" class="error" role="alert">
      比較に失敗しました。再実行してください。{{ error }}
    </p>
    <template v-if="results">
      <SummaryCard label="入力した文章" :stats="results.totals" />
      <div class="filter-bar" role="group" aria-label="結果フィルタ">
        <label class="filter-check">
          <input v-model="showMatch" type="checkbox" />
          <span class="badge badge-match">一致</span>
        </label>
        <label class="filter-check">
          <input v-model="showLight" type="checkbox" />
          <span class="badge badge-light">軽微な差異</span>
        </label>
        <label class="filter-check">
          <input v-model="showFatal" type="checkbox" />
          <span class="badge badge-fatal">差異あり</span>
        </label>
        <label class="filter-check">
          <input v-model="showError" type="checkbox" />
          <span class="badge badge-error">エラー</span>
        </label>
      </div>
      <EntryList
        :files="results.files"
        :selected-file="null"
        :show-match="showMatch"
        :show-light="showLight"
        :show-fatal="showFatal"
        :show-error="showError"
      />
    </template>
  </section>
</template>

<style scoped>
.text-comparison {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 32px;
}
h2 {
  font-size: 16px;
}
form {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
textarea,
button {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-surface);
  color: var(--color-text);
  font: inherit;
  padding: 10px 12px;
}
textarea {
  width: 100%;
  resize: vertical;
  min-height: 100px;
}
textarea:focus-visible,
button:focus-visible {
  outline: 2px solid var(--color-match);
  outline-offset: 2px;
}
button {
  min-height: 44px;
  cursor: pointer;
}
button:enabled:hover {
  background: var(--color-surface2);
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}
.error {
  color: var(--color-fatal);
  overflow-wrap: anywhere;
}
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.filter-check {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
</style>
