# A price read from a CSV file is a string. `*` is happy anyway.

def line_total(quantity, price):
    return quantity * price

print("2 x 250 =", line_total(2, 250))
print("2 x '250' =", line_total(2, "250"))      # "250250"
