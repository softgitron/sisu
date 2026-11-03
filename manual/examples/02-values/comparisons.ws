#!/usr/bin/env sisu
# Comparisons give a `bool`.
3 == 3  # => True
3 != 4  # => True
2.5 < 3.0  # => True
"Aino" < "Eero"  # => True
'a' < 'b'  # => True
let age = 34  # => 34
18 <= age < 65  # => True
0 < age < 18  # => False
--- stderr
--- status 0
