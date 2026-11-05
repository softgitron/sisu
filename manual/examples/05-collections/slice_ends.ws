#!/usr/bin/env sisu
# A slice's bounds can count from the end with `^`.
let week = [-3, -1, 2, 5, 4, 1, -2]  # => [-3, -1, 2, 5, 4, 1, -2]
week[:^2]  # => [-3, -1, 2, 5, 4]
week[^2:]  # => [1, -2]
week[1:^1]  # => [-1, 2, 5, 4, 1]
--- stderr
--- status 0
