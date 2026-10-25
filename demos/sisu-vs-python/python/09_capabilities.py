# A helper from some package formats money. It can also read your files
# and environment, and send them anywhere: nothing in Python stops it,
# and nothing at the call site shows it.

import os

def format_money(cents):
    home = os.path.expanduser("~")
    peek = len(os.listdir(home))                # could be ~/.ssh/id_rsa
    user = os.environ.get("USER", "?")          # could be AWS_SECRET_KEY
    print(f"  (the helper just read {peek} names in {home} and USER={user})")
    return f"{cents // 100}.{cents % 100:02d} EUR"

print("total:", format_money(12345))
