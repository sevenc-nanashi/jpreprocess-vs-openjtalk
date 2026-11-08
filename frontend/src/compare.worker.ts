import type { Results } from "./types";

type ComparisonModule = {
  FS: {
    writeFile(path: string, text: string): void;
    readFile(path: string, options: { encoding: "utf8" }): string;
  };
  callMain(args: string[]): number;
};

export type ComparisonResponse = { results: Results } | { error: string };

self.onmessage = async (event: MessageEvent<{ text: string; baseUrl: string }>) => {
  try {
    const { text, baseUrl } = event.data;
    const moduleUrl = new URL("jpreprocess_vs_openjtalk.js", baseUrl).href;
    const { default: createModule } = await import(/* @vite-ignore */ moduleUrl);
    const module: ComparisonModule = await createModule({
      noInitialRun: true,
      locateFile: (file: string) => new URL(file, baseUrl).href,
      print: () => {},
    });
    module.FS.writeFile("/input.txt", text);
    const code = module.callMain(["--json", "/results.json", "/input.txt"]);
    if (code !== 0) throw new Error(`比較処理が終了しました (exit ${code})`);
    const results: Results = JSON.parse(module.FS.readFile("/results.json", { encoding: "utf8" }));
    self.postMessage({ results } satisfies ComparisonResponse);
  } catch (error) {
    self.postMessage({ error: String(error) } satisfies ComparisonResponse);
  }
};
