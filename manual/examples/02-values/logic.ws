#!/usr/bin/env sisu
# `and`, `or` and `not` combine `bool` values.
let raining = True  # => True
let have_umbrella = False  # => False
raining and not have_umbrella  # => True
raining or have_umbrella  # => True
not raining  # => False
(raining and have_umbrella) or not raining  # => False
--- stderr
--- status 0
