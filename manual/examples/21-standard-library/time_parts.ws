#!/usr/bin/env sisu
# A `DateTime` gives its parts, in UTC.
let t = DateTime.parse("2026-11-05T17:30:15Z").expect("a time")  # => 2026-11-05T17:30:15Z
t.year()  # => 2026
t.month()  # => 11
t.day()  # => 5
t.hour()  # => 17
t.minute()  # => 30
t.weekday()  # => 4
t.day_of_year()  # => 309
--- stderr
--- status 0
