#!/usr/bin/env sisu
# Comprehensions build a list, a set or a dict from a loop.
let scores = [7, 9, 4, 9]  # => [7, 9, 4, 9]
[s * 10 for s in scores]  # => [70, 90, 40, 90]
[s for s in scores if s > 5]  # => [7, 9, 9]
{s for s in scores}  # => {7, 9, 4}
{name: len(name) for name in ["Aino", "Liisa"]}  # => {"Aino": 4, "Liisa": 5}
--- stderr
--- status 0
