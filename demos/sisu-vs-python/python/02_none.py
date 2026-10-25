# A lookup that can find nothing. Python lets the caller forget that.

customers = {"aino": {"discount": 10}, "roni": {"discount": 5}}

def find_customer(name):
    return customers.get(name)                  # None when absent

for name in ["aino", "roni", "mika"]:           # mika is new
    print(name, "gets", find_customer(name)["discount"], "% off")
