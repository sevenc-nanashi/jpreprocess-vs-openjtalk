# 追加コーパスの出典とライセンス

以下の6件について、公開版の会話・シナリオ本文を収録しています。Rosebleu は成人向けシーンを除外し、ほかの5件は全文を対象にしています。
抽出データと比較結果中の原文には、それぞれ元データのライセンスが適用されます。
このリポジトリのプログラムのライセンスで置き換えるものではありません。

| 比較ファイル | 出典・著作者／公開者 | ライセンス | 元ファイル数 | 抽出発話数 |
| --- | --- | --- | ---: | ---: |
| `character_conversation.txt` | [Character Conversation Dataset](https://github.com/matsuvr/characterconversationdataset) — matsu_vr | [Apache-2.0](licenses/character_conversation/LICENSE) | 5 | 787 |
| `rosebleu.txt` | [Rosebleu ゲームシナリオ](https://gitlab.com/open_contents_datasets/Rosebleu) — © Rosebleu、青猫提供、Open Contents Datasets 公開 | [Apache-2.0](licenses/rosebleu/LICENSE) | 1,733 | 141,685 |
| `ojousama_talk.txt` | [お嬢様会話データセット](https://github.com/matsuvr/OjousamaTalkScriptDataset) — © 2023 matsu_vr | [MIT](licenses/ojousama_talk/LICENSE) | 1 | 404 |
| `real_persona_chat.txt` | [RealPersonaChat](https://github.com/nu-dialogue/real-persona-chat) — 山下紗苗、井上昂治、郭傲、望月翔太、河原達也、東中竜一郎／nu-dialogue | [CC BY-SA 4.0](licenses/real_persona_chat/LICENSE) | 13,583 | 408,619 |
| `jmrd.txt` | [Japanese Movie Recommendation Dialogue](https://github.com/ku-nlp/JMRD) — 児玉貴志、田中リベカ、黒橋禎夫／ku-nlp | [CC BY-SA 4.0](licenses/jmrd/LICENSE) | 3 | 114,750 |
| `mrmp.txt` | [Multi-Relational Multi-Party Chat Corpus](https://github.com/nu-dialogue/multi-relational-multi-party-chat-corpus) — 津田太郎、山下紗苗、井上昂治、河原達也、東中竜一郎／nu-dialogue | [CC BY-SA 4.0](licenses/mrmp/LICENSE) | 960 | 100,680 |

合計 **766,925 発話**。句点・かぎ括弧でさらに分割するため、比較時の文数とは異なります。
Rosebleu は元の README の区別に従い、ファイル名が `h_converted.jsonl` で終わる72ファイル・15,855発話を除外しています（大文字の `H` も対象）。通常シーン内の表現を個別に判定する処理は行っていません。

## 固定したリビジョン

- Character Conversation Dataset: `b7f185a89d9f39b06f2d11a0040ca6956e4dbd78`
- Rosebleu: `b8a169893c87611c7c1c76d78acd1cbe3273a146`
- お嬢様会話データセット: `589f3b52324cc12ad3fb0b2ebe1520bbffce4087`
- RealPersonaChat: `28d0b6b3865b29cabc26c230a2db37cdf315e937`
- JMRD: `a20b0a89b4cf6ce1db320039f6e27df10604bf4c`
- MRMP: `e6e39cb896df88781c7a2e0451226a527c5f1e2e`

## 抽出方法

`data/import_conversations.rb` により次の加工を行っています。

- Character Conversation Dataset は5つの JSONL の `text`、Rosebleu は上記の成人向けシーンを除く JSONL の `utterance` を抽出します。Rosebleu の TSV は同じデータの別形式なので重複収録しません。
- お嬢様会話データセットは CSV の `prompt` と `completion` を交互に抽出します。JSONL の学習用マーカーや別途 GPT で生成されたバリエーションは取り込みません。
- RealPersonaChat・MRMP は公開対話の `utterances[].text` のみを抽出します。話者属性・ペルソナ・評価、RealPersonaChat の追加 LLM 実験データは含めません。
- JMRD は train・valid・test の全対話の `dialog[].text` を抽出します。外部知識・アンケート項目は含めません。
- ファイル名順・元の発話順を保ち、抽出数の上限や重複除去は設けません。発話内の改行は空白に置き換え、前後の空白を除き、空の発話を除外します。
- 発話末尾に句点がなければ `。` を追加します。比較プログラムが改行を取り除いても別の発話と連結しないための区切りです。
- 元のマスク表記・誤字・記号は維持します。比較時の空白除去やライブラリによる正規化は別の処理です。

## 再取得

リポジトリのルートから実行します。Ruby と CSV ライブラリ、ネットワーク接続が必要です。
生成済みの比較ファイルとライセンスを置き換えます。

```sh
ruby data/import_conversations.rb
# 1件だけ再生成する場合
ruby data/import_conversations.rb real_persona_chat
```

固定コミットを変更して更新する場合は、この文書のリビジョン・件数・ライセンスも確認してください。

## 原典の文献・注意事項

RealPersonaChat・MRMP のマスク済み個人情報を推定したり、話者のなりすましに利用したりしないでください。
各コーパスの文献と公開方針は、それぞれのリポジトリの README に記載されています。
CC BY-SA 4.0 の条件は [Creative Commons のライセンス説明](https://creativecommons.org/licenses/by-sa/4.0/deed.ja)も参照してください。
