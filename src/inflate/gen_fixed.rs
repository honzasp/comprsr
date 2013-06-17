def bl_count(len)
  case len
  when 7: 279-256+1
  when 8: 143+287-280+2
  when 9: 255-144+1
  else 0
  end
end

def code_len(code)
  case code
  when (0..143): 8
  when (144..255): 9
  when (256..279): 7
  when (280..287): 8
  end
end

def rev5(x)
  ((x & 0b10000) >> 4) +
    ((x & 0b1000) >> 2) +
    ((x & 0b100)) +
    ((x & 0b10) << 2) +
    ((x & 0b1) << 4)
end

code = 0
next_code = (0..9).map { 0 }
(1..9).each do |bits|
  code = (code + bl_count(bits-1)) * 2
  next_code[bits] = code
end

codes = (0..287).map do |n|
  len = code_len(n)
  next_code[len] += 1
  next_code[len] - 1
end

rev_prefixes = []
codes.each_with_index do |code, n|
  prefix = code >> (code_len(n) - 5)
  rev_prefix = rev5 prefix
  if !rev_prefixes[rev_prefix]
    rev_prefixes[rev_prefix] = true
    #puts "#{rev_prefix.to_s(2).rjust(5,"0")}: (#{n},#{code_len(n) - 5}),"
    print "(#{n},#{code_len(n) - 5}), "
  end
end
puts
