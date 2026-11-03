#!/usr/bin/env sisu
# The common methods of text.
let line = "  Hei, Maailma!  "  # => "  Hei, Maailma!  "
line.trim()  # => "Hei, Maailma!"
line.trim().to_lowercase()  # => "hei, maailma!"
line.trim().starts_with("Hei")  # => True
line.trim().replace("Maailma", "Aino")  # => "Hei, Aino!"
let at = line.find("M")  # => Some(7)
let missing = line.find("x")  # => None
"v1.2.0".strip_prefix("v")  # => "1.2.0"
[part for part in "red,green,,blue".split(",")]  # => ["red", "green", "", "blue"]
[word for word in "one two\tthree".split_whitespace()]  # => ["one", "two", "three"]
[row for row in "first\nsecond".lines()]  # => ["first", "second"]
--- stderr
--- status 0
