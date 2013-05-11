#!/usr/bin/ruby
require 'zlib'
test_path = File.dirname(__FILE__)

Dir.foreach test_path do |filename|
  if File.extname(filename) == ".out"
    STDOUT.print "#{filename}... "
    STDOUT.flush

    zlib_file = "#{File.basename filename, ".out"}.zlib"
    unless File.exists? zlib_file
      STDOUT.print "zlib... "
      STDOUT.flush
      File.open zlib_file, "w" do |f|
        f.write Zlib::Deflate.deflate(File.read filename)
      end
    end

    STDOUT.puts "ok"
    STDOUT.flush
  end
end
