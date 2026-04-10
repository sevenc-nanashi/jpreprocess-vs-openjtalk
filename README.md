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
- `kokoro.txt`：こころ（夏目漱石）。
- `wagahaiwa_nekodearu.txt`：吾輩は猫である（夏目漱石）。

## 比較方法

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

```
cargo run --release -- --json frontend/public/results.json ./data/*.txt
cd frontend && vp install && vp dev
```
