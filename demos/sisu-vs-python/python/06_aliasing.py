# Two names, one list. Appending to the report changes today's orders.

todays_orders = [1200, 500, 800]
report = todays_orders                          # not a copy
report.append(sum(report))                      # a totals row

print("orders today:", len(todays_orders))      # 4, should be 3
print("revenue:", sum(todays_orders))           # doubled
