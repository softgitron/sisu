# "Use the customer's discount, or 10% if they have none."
# A discount of 0 is falsy, so it silently becomes 10%.

customers = [("aino", 15), ("roni", None), ("mika", 0)]

for name, discount in customers:
    percent = discount or 10
    print(name, "gets", percent, "% off")       # mika: 10, should be 0
