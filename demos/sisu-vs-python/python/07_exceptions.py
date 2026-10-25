# Nothing in `parse_quantity`'s signature says it can fail, so the
# caller does not handle it, and the batch dies half done.

def parse_quantity(text):
    return int(text)

charged = []
for line in ["2", "1", "two", "3"]:
    charged.append(parse_quantity(line))
    print("charged card for", charged[-1], "items")
print("batch complete")
