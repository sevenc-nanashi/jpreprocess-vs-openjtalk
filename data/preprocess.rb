# frozen_string_literal: true

files = ARGV
files.each do |file|
  File.open(file, "rb") do |f|
    destination = file.gsub(".raw", "")
    raise "Destination file is the same as source file" if destination == file
    content = f.read.force_encoding("shift_jis").encode("utf-8")
    content.gsub!("\r\n", "\n")
    content.gsub!(/［＃[０-９]+字下げ］.+$/, "")
    content.gsub!(/｜/, "")
    content.gsub!(/［.+?］/, "")
    content.gsub!(/《.+?》/, "")
    content.gsub!(/.+-\n\n/m, "")
    content.gsub!(/底本：.+/m, "")

    File.binwrite(destination, content)
    puts "#{file} -> #{destination}"
  end
end
