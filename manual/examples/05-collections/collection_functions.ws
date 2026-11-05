#!/usr/bin/env sisu
# The prelude's functions for collections.
let scores = [7, 9, 4, 9]  # => [7, 9, 4, 9]
len(scores)  # => 4
sorted(scores)  # => [4, 7, 9, 9]
sum(scores)  # => 29
max(4, 9)  # => 9
max(scores)  # => 9
min(scores)  # => 4
any([s > 8 for s in scores])  # => True
all([s > 5 for s in scores])  # => False
--- stderr
--- status 0
