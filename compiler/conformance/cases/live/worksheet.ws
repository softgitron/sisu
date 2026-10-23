#!/usr/bin/env sisu
# A worksheet (10, "REPL and scripts"; Q-48): `sisu ws` runs the script
# from the top and writes each top-level statement's value and output
# after it, as comments. It runs against a test's fake `System`: this
# file is written to memory, not to the disk.

def double(n: int) -> int:
    return n * 2

let x = 1 + 2  # => 3
let ys = [x, double(x)]  # => [3, 6]
ys.len()  # => 2
print("ys is {ys}")
# | ys is [3, 6]
for y in ys:
    print(y * 10)
# | 30
# | 60
sys.fs.write_text("note.txt", "kept in memory").expect("written")
sys.fs.read_to_string("note.txt").expect("read")  # => "kept in memory"
sys.fs.exists("worksheet.sisu")  # => False
--- stderr
--- status 0
