# A status gains a new value. Every if/elif chain over it is now wrong,
# and Python says nothing: the function falls off its end and gives None.

def label(status):
    if status == "paid":
        return "Paid, packing it"
    elif status == "shipped":
        return "On its way"

for status in ["paid", "shipped", "refunded"]:  # "refunded" is new
    print(status, "->", label(status))
