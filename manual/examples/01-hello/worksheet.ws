#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes the value of each line after it.
let name = "Aino"  # => "Aino"
let greeting = "Hei, {name}!"  # => "Hei, Aino!"
print(greeting)
# | Hei, Aino!
len(greeting)  # => 10
greeting.to_uppercase()  # => "HEI, AINO!"
--- stderr
--- status 0
