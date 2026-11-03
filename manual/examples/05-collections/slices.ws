#!/usr/bin/env sisu
# A slice copies a run of a list's elements.
let week = [-3, -1, 2, 5, 4, 1, -2]  # => [-3, -1, 2, 5, 4, 1, -2]
week[0:5]  # => [-3, -1, 2, 5, 4]
week[5:]  # => [1, -2]
week[:2]  # => [-3, -1]
len(week[2:4])  # => 2
--- stderr
--- status 0
