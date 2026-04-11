# frozen_string_literal: true
#
# https://github.com/mmorise/rohan4600 のコーパスをダウンロードして文章のみを抽出するスクリプト
require "open-uri"

url = "https://raw.githubusercontent.com/mmorise/rohan4600/refs/heads/main/Rohan4600_transcript_utf8.txt"
destination = "rohan4600_transcript_utf8.txt"

URI.open(url) do |f|
  content = f.read
  File.binwrite(destination, content.gsub(/ROHAN4600_[0-9]+:/, "").gsub(/\(.+?\)/, "").gsub(/,.+$/, ""))
  puts "Downloaded #{url} to #{destination}"
end
