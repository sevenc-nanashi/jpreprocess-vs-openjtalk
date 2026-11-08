# frozen_string_literal: true

require "csv"
require "fileutils"
require "json"
require "open-uri"
require "rubygems/package"
require "tempfile"
require "zlib"

SOURCES = {
  "character_conversation" => {
    url: "https://codeload.github.com/matsuvr/characterconversationdataset/tar.gz/b7f185a89d9f39b06f2d11a0040ca6956e4dbd78",
    files: /\A(?:alice|betty|catherine|common|daisy)\.jsonl\z/,
    format: :jsonl, field: "text"
  },
  "rosebleu" => {
    url: "https://gitlab.com/api/v4/projects/50846790/repository/archive.tar.gz?sha=b8a169893c87611c7c1c76d78acd1cbe3273a146",
    # The source README marks adult scenes with a trailing h in the scenario name.
    files: /(?<!h)_converted\.jsonl\z/i,
    format: :jsonl, field: "utterance"
  },
  "ojousama_talk" => {
    url: "https://codeload.github.com/matsuvr/OjousamaTalkScriptDataset/tar.gz/589f3b52324cc12ad3fb0b2ebe1520bbffce4087",
    files: /\Aojousamatalkscript200\.csv\z/,
    format: :csv
  },
  "real_persona_chat" => {
    url: "https://codeload.github.com/nu-dialogue/real-persona-chat/tar.gz/28d0b6b3865b29cabc26c230a2db37cdf315e937",
    files: %r{\Areal_persona_chat/dialogues/[^/]+\.json\z},
    format: :dialogue
  },
  "jmrd" => {
    url: "https://codeload.github.com/ku-nlp/JMRD/tar.gz/a20b0a89b4cf6ce1db320039f6e27df10604bf4c",
    files: %r{\Adata/(?:train|valid|test)\.json\z},
    format: :jmrd
  },
  "mrmp" => {
    url: "https://codeload.github.com/nu-dialogue/multi-relational-multi-party-chat-corpus/tar.gz/e6e39cb896df88781c7a2e0451226a527c5f1e2e",
    files: %r{\Amulti_relational_multi_party_chat_corpus/dialogues/(?:A_first_time|B_family)/[^/]+\.json\z},
    format: :dialogue
  }
}.freeze

names = ARGV.empty? ? SOURCES.keys : ARGV
names.each { |name| SOURCES.fetch(name) }

names.each do |name|
  source = SOURCES.fetch(name)
  files = {}
  warn "Downloading #{name}"
  URI.open(source.fetch(:url), open_timeout: 30, read_timeout: 120) do |archive|
    Zlib::GzipReader.wrap(archive) do |gzip|
      Gem::Package::TarReader.new(gzip) do |tar|
        tar.each do |entry|
          next unless entry.file?

          path = entry.full_name.split("/", 2).fetch(1)
          next unless path == "LICENSE" || source.fetch(:files).match?(path)

          files[path] = entry.read.force_encoding(Encoding::UTF_8)
        end
      end
    end
  end
  license = files.delete("LICENSE")
  raise "Missing LICENSE for #{name}" unless license
  raise "No source files for #{name}" if files.empty?

  count = 0
  destination = File.join(__dir__, "#{name}.txt")
  Tempfile.create([name, ".txt"], __dir__) do |output|
    emit = lambda do |text|
      text = text.gsub(/\r\n|\r|\n/, " ").strip
      next if text.empty?

      # The comparison CLI splits at 。, not newlines; keep utterances independent.
      output.puts(text.end_with?("。") ? text : "#{text}。")
      count += 1
    end

    files.sort.each do |_path, content|
      case source.fetch(:format)
      when :jsonl
        content.each_line do |line|
          next if line.strip.empty?

          emit.call(JSON.parse(line).fetch(source.fetch(:field)))
        end
      when :csv
        CSV.parse(content, headers: true).each do |row|
          emit.call(row.fetch("prompt"))
          emit.call(row.fetch("completion"))
        end
      when :dialogue
        JSON.parse(content).fetch("utterances").each { |utterance| emit.call(utterance.fetch("text")) }
      when :jmrd
        JSON.parse(content).each do |dialogue|
          dialogue.fetch("dialog").each { |utterance| emit.call(utterance.fetch("text")) }
        end
      else
        raise "Unknown format for #{name}"
      end
    end
    raise "No utterances for #{name}" if count.zero?

    output.close
    File.rename(output.path, destination)
  end
  license_dir = File.join(__dir__, "licenses", name)
  FileUtils.mkdir_p(license_dir)
  File.write(File.join(license_dir, "LICENSE"), license)
  puts "#{name}: #{files.size} files, #{count} utterances -> #{destination}"
end
