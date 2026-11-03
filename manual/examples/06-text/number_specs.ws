#!/usr/bin/env sisu
# Format specs for integers.
let n = 42  # => 42
"[{n:5}] [{n:<5}] [{n:^5}] [{n:05}]"  # => "[   42] [42   ] [ 42  ] [00042]"
"{n:x} {n:#x} {n:b} {n:o}"  # => "2a 0x2a 101010 52"
"{1234567:,}"  # => "1,234,567"
--- stderr
--- status 0
