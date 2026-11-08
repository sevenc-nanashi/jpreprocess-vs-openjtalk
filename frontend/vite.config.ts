import { defineConfig, lazyPlugins } from "vite-plus";
import vize from "@vizejs/vite-plugin";

export default defineConfig({
  fmt: {
    ignorePatterns: ["dist/**", "public/results.json", "public/result-pages/**", "public/wasm/**"],
  },
  lint: {
    ignorePatterns: ["**/node_modules/**", "dist/**", "public/result-pages/**", "public/wasm/**"],
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  plugins: lazyPlugins(() => [vize()]),
  base: "./",
  worker: { format: "es" },
});
