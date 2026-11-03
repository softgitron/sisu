#!/usr/bin/env sisu
# Numbers out of text: the result is optional.
let answer = "42".parse_int()  # => Some(42)
let below_zero = "-7".parse_int()  # => Some(-7)
let not_a_number = "4x".parse_int()  # => None
let ratio = "2.5".parse_float()  # => Some(2.5)
let padded = " 42 ".trim().parse_int()  # => Some(42)
--- stderr
--- status 0
