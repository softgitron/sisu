#!/usr/bin/env sisu
# Wrapping and saturating arithmetic, spelled out.
let largest = 9_223_372_036_854_775_807  # => 9223372036854775807
largest +% 1  # => -9223372036854775808
largest +| 1  # => 9223372036854775807
let level = 250u8  # => 250
level +% 10  # => 4
level +| 10  # => 255
level -| 251  # => 0
--- stderr
--- status 0
