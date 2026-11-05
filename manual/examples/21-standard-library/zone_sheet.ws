#!/usr/bin/env sisu
# A worksheet shows a `Zone` as the code that makes it.
from std.time import Zone
let helsinki = Zone.named("Europe/Helsinki").expect("a zone")  # => Zone.named("Europe/Helsinki")
let india = Zone.fixed(19800)  # => Zone.fixed(19800)
let noon = DateTime.parse("2026-07-01T12:00:00Z").expect("a time")  # => 2026-07-01T12:00:00Z
india.format(noon)  # => "2026-07-01T17:30:00+05:30"
helsinki.abbreviation_at(noon)  # => "EEST"
--- stderr
--- status 0
