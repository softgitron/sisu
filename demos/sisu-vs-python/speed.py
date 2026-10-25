# The same work as speed.sisu: price N generated orders with discounts,
# in whole cents, and print a checksum that must match Sisu's.
import sys

def run(n):
    x = 42
    total = 0
    for _ in range(n):
        x = (x * 1103515245 + 12345) % 2147483648
        quantity = x % 7 + 1
        price = x // 7 % 5000 + 99
        discount = x // 35000 % 4 * 5
        gross = quantity * price
        net = gross - gross * discount // 100
        total = (total * 31 + net) % 1000000007
    return total

n = int(sys.argv[1]) if len(sys.argv) > 1 else 5_000_000
print("checksum", run(n))
