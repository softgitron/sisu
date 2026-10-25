# A typo in a branch that rarely runs. Python only finds it when the
# branch runs: here, at the shop's first refund, months after release.

class Order:
    def __init__(self, quantity, price_cents, refunded=False):
        self.quantity = quantity
        self.price_cents = price_cents
        self.refunded = refunded

def line_total(o):
    if o.refunded:
        return -o.quantitiy * o.price_cents     # typo
    return o.quantity * o.price_cents

orders = [Order(2, 500), Order(1, 1200)]
print("today's total:", sum(line_total(o) for o in orders))
orders.append(Order(1, 500, refunded=True))    # the first refund
print("today's total:", sum(line_total(o) for o in orders))
