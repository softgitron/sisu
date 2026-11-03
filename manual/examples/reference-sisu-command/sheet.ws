#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes each statement's value after it.
let prices = [12, 5, 30]  # => [12, 5, 30]
let total = sum(prices)  # => 47
print("total {total}")
# | total 47
total // len(prices)  # => 15
total.to_f64() / 3.0  # => 15.666666666666666
--- stderr
--- status 0
