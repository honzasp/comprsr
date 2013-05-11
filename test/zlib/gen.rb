#!/usr/bin/ruby
require 'zlib'
test_path = File.dirname(__FILE__)

Dir.foreach test_path do |filename|
  if File.extname(filename) == ".out"
    STDOUT.print "#{filename}... "
    STDOUT.flush

    zlib_file = "#{File.basename filename, ".out"}.zlib"
    zlib_path = File.join test_path, zlib_file
    out_path = File.join test_path, filename
    unless File.exists? zlib_path
      STDOUT.print "zlib... "
      STDOUT.flush
      File.open zlib_path, "w" do |f|
        f.write Zlib::Deflate.deflate(File.read out_path)
      end
    end

    STDOUT.puts "ok"
    STDOUT.flush
  end
end
