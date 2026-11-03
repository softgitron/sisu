#!/usr/bin/env sisu
# `in` asks whether a collection holds a value.
let primes = [2, 3, 5, 7]  # => [2, 3, 5, 7]
5 in primes  # => True
4 in primes  # => False
4 not in primes  # => True
let ages = {"Aino": 34, "Eero": 29}  # => {"Aino": 34, "Eero": 29}
"Aino" in ages  # => True
"in" in "Aino"  # => True
--- stderr
--- status 0
