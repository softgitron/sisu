# Splitting a bill in cents. `/` quietly turns money into a float.

total_cents = 1000
people = ["aino", "roni", "mika"]
share = total_cents / len(people)
print("each pays", share, "cents")              # 333.3333333333333
print("collected", int(share) * len(people), "of", total_cents)  # a cent lost
