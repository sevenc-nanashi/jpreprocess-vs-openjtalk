# frozen_string_literal: true

if ARGV.length != 3
  puts "Usage: ruby filter.rb <log> <target> <output>"
  puts "target: light | fatal | jp_error | ojt_error"
  exit
end

log = ARGV[0]
target = ARGV[1]
output = ARGV[2]

File.open(output, "w") do |output_file|
  in_entry = false
  current_level = ""
  File.open(log, "r") do |file|
    file.each_line do |line|
      if line[0] == "["
        in_entry = true
        if line.include? "Light"
          current_level = "light"
        elsif line.include? "Fatal"
          current_level = "fatal"
        elsif line.include? "OpenJTalk Error"
          current_level = "jp_error"
        elsif line.include? "JPreprocess"
          current_level = "ojt_error"
        else
          raise "Unknown level"
        end
        output_file.puts line if current_level == target
      elsif line[0] != " "
        break
      elsif in_entry && current_level == target
        output_file.puts line
      end
    end
  end
end
