# openjtalk-vs-jpreprocess

比較用。

最新の比較結果は GitHub Pages で公開されています：
<https://sevenc-nanashi.github.io/jpreprocess-vs-openjtalk/>

## 比較対象

- `bridges.txt`：[Coeiroink v2とA.I.VoiceをVoicevox上で使えるようにしてみた【COEIROINK投稿祭2023】](https://www.nicovideo.jp/watch/sm43073706?ref=garage_share_other) の台本。
- `cantari.txt`：[Cantari - UTAU音源をVoicevox上で喋らせるツールを作ってみた](https://www.nicovideo.jp/watch/sm43856969?ref=garage_share_other) の台本。
- `chuni_01.txt`：昔作ったChunithm実況（Short）の台本。（動画は無くした）
- `chuni_02.txt`：[もち子さんはチョコミントをAJしたい #Chunithm](https://www.nicovideo.jp/watch/sm42625985?ref=garage_share_other) の台本。
- `dhmo_ansaikuropedia.txt`：[DHMO - アンサイクロペディア](https://ansaikuropedia.org/wiki/DHMO)。
- `dnc_ansaikuropedia.txt`：[大学入試センター試験 - アンサイクロペディア](https://ansaikuropedia.org/wiki/%E5%A4%A7%E5%AD%A6%E5%85%A5%E8%A9%A6%E3%82%BB%E3%83%B3%E3%82%BF%E3%83%BC%E8%A9%A6%E9%A8%93)。
- `extreme_apologize_ansaikuropedia.txt`：[エクストリーム・謝罪 - アンサイクロペディア](https://ansaikuropedia.org/wiki/%E3%82%A8%E3%82%AF%E3%82%B9%E3%83%88%E3%83%AA%E3%83%BC%E3%83%A0%E3%83%BB%E8%AC%9D%E7%BD%AA)。
- `rohan4600_transcript_utf8.txt`：[ROHAN](https://github.com/mmorise/rohan4600) コーパス。
- `kokoro.txt`：こころ（夏目漱石）。
- `wagahaiwa_nekodearu.txt`：吾輩は猫である（夏目漱石）。

## 比較方法

両ライブラリの辞書は、共通の [naist-jdic v0.1.3](https://github.com/jpreprocess/naist-jdic/tree/v0.1.3) からビルド時に生成します。
辞書ソースを一度ダウンロードし、同じ CSV・文字種定義・接続コストをそれぞれの辞書形式に変換します。
生成先は Cargo の `OUT_DIR` です。初回ビルドにはネットワーク接続と辞書の生成時間が必要です。

- ctee：<https://github.com/Lipen/ctee>
```
cargo run ./data/wagahaiwa_nekodearu.txt ./data/kokoro.txt | tee ./output.ansi.log | ctee ./output.log
```

手っ取り早く全部のファイルを比較する場合は、以下のようにする。
```
cargo run ./data/*.txt | tee ./output.ansi.log | ctee ./output.log
```

JSON を出力する場合は `--json <path>` を追加する。
```
cargo run --release -- --json frontend/public/results.json ./data/*.txt
```

## フロントエンドのローカル起動

ブラウザの「文章を比較」から任意の文章を比較できます。処理は Web Worker 内の
WebAssembly で実行され、入力した文章はサーバーに送信されません。
初回実行時に両ライブラリの辞書（非圧縮で約 179 MiB）をダウンロードします。
辞書のライセンスは [naist-jdic-COPYING.txt](frontend/public/naist-jdic-COPYING.txt) に収録しています。

比較結果では、両ライブラリの表記と発音をアクセント句ごとのルビで表示します。
区切りは厳密な文法上の文節ではなく、各ライブラリの前処理結果に従います。
数字などの表記は正規化される場合があるため、入力した原文も併記します。
読みの `’` は母音の無声化を表します。
句の読み違いは赤、無声化だけの違いは黄色で表示し、同じ句に複数の違いがある場合は赤を優先します。
既存の `results.json` に読みを表示するには、上記のコマンドで再生成してください。

ブラウザ版のビルドには Emscripten SDK 5.0.7 と Rust の
`wasm32-unknown-emscripten` ターゲットが必要です。
`open_jtalk-rs` は [PR #70](https://github.com/VOICEVOX/open_jtalk-rs/pull/70)
のコミット `7e560e1` に固定しています。

```
source /path/to/emsdk/emsdk_env.sh
rustup target add wasm32-unknown-emscripten
bash scripts/build-wasm.sh
cargo run --release -- --json frontend/public/results.json ./data/*.txt
cd frontend && vp install && vp dev
```

既存の比較結果の閲覧だけなら WebAssembly のビルドは不要です。
