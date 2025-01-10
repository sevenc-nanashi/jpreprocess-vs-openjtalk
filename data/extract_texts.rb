# frozen_string_literal: true
require "json"

files = ARGV
files.each do |file|
  content = JSON.parse(File.read(file))
  destination = file.gsub(".vvproj", ".txt")

  audio_keys = content["talk"]["audioKeys"]
  lines =
    audio_keys
      .map do |audio_key|
        "#{content["talk"]["audioItems"][audio_key]["text"]}。"
      end
      .join("\n")

  File.write(destination, lines)

  puts "#{file} -> #{destination}"
end
