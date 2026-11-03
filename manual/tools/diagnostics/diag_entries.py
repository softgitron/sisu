# The entries of R17: (code, stem, wrong, fixed, rule, fix).
# `wrong` is the program with the mistake; `fixed` the program with the
# fix; `rule` says what the code guards; `fix` reads the fix for the reader.
E = []
def entry(code, stem, wrong, fixed, rule, fix, kind="compile"):
    E.append(dict(code=code, stem=stem, wrong=wrong, fixed=fixed, rule=rule, fix=fix, kind=kind))

entry("E0001", "e0001_tab",
"""# A tab in the indentation.
def main():
\tprint("Hei, maailma!")
""",
"""# Indented with four spaces.
def main():
    print("Hei, maailma!")
""",
"A line is indented with spaces only. A tab in the indentation is an error wherever it stands, since its width is a matter of the editor.",
"The note asks for spaces in place of each tab; `sisu fmt` cannot do it, because it formats only files that parse.")

entry("E0003", "e0003_open_string",
"""# A string literal that is not closed.
def main():
    print("Hei, maailma!)
""",
"""# The string literal closed on its line.
def main():
    print("Hei, maailma!")
""",
"A one-line string literal ends on the line where it starts. A multi-line literal is written between `\"\"\"` marks.",
"The fix closes the literal on its line, before the brackets that the line opened ahead of the literal: here before the `)`, where the quote belongs.")

entry("E0016", "e0016_char_literal",
"""# Single quotes around a word.
def main():
    let name = 'Aino'
    print(name)
""",
"""# Text between double quotes.
def main():
    let name = "Aino"
    print(name)
""",
"Single quotes make a `char`, one character; text is written between double quotes.",
"The diagnostic offers no fix. Use double quotes for text, and keep single quotes for one character, such as `'A'`.")

entry("E0100", "e0100_no_colon",
"""# A block opened without its colon.
def main()
    print("Hei, maailma!")
""",
"""# The colon that opens the block.
def main():
    print("Hei, maailma!")
""",
"The parser expected a particular token and found another. The message names both.",
"The fix writes the missing token where the parser expected it, the `:` that opens the body here.")

entry("E0103", "e0103_no_operand",
"""# An operator without its right operand.
def main():
    let total = 3 +
    print(total)
""",
"""# Both operands, on one line.
def main():
    let total = 3 + 4
    print(total)
""",
"An expression was expected, and something else stood there: the end of the line, a keyword, or a statement. A line ends a statement unless a bracket is open.",
"There is no fix to offer, since only you know the missing expression. To break a long expression, break it inside brackets.")

entry("E0105", "e0105_pattern",
"""# `not` in a pattern.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case not 0: "other"

def main():
    print(describe(0))
""",
"""# `_` matches every other value.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case _: "other"

def main():
    print(describe(0))
""",
"A pattern was expected: a literal, a name, `_`, a variant, a tuple, a list or a struct pattern, or alternatives with `|`. An operator such as `not` is not a pattern.",
"Write the case as a pattern; a `case _` arm, or a guard `case n if n != 0`, says \"any other value\".")

entry("E0108", "e0108_and_or",
"""# `and` and `or` mixed without parentheses.
def main():
    let raining = True
    let cold = False
    let windy = True
    if raining and cold or windy:
        print("stay in")
""",
"""# Parentheses say which goes first.
def main():
    let raining = True
    let cold = False
    let windy = True
    if (raining and cold) or windy:
        print("stay in")
""",
"`and` and `or` do not mix without parentheses, so that no reader has to remember which binds tighter.",
"Add the parentheses that say what you mean.")

entry("E0113", "e0113_chain",
"""# A chained comparison that points two ways.
def main():
    let size = 3
    print(0 < size > 5)
""",
"""# A chain that points one way.
def main():
    let size = 3
    print(0 < size < 5)
""",
"A chain of comparisons such as `0 < x <= 5` reads as one range, so all its operators point the same way.",
"Write the chain one way, or join two comparisons with `and`.")

entry("E0115", "e0115_python_if",
"""# Python's conditional expression.
def main():
    let size = 3
    let label = "big" if size > 2 else "small"
    print(label)
""",
"""# Sisu's `if` expression.
def main():
    let size = 3
    let label = if size > 2: "big" else: "small"
    print(label)
""",
"Sisu's conditional expression starts with `if`: `if c: a else: b`. Python's `a if c else b` does not compile, so that it never means something else (D-062).",
"The fix rewrites the expression in Sisu's order, which is what you want.")

entry("E0116", "e0116_if_without_else",
"""# An `if` expression without an `else`.
def main():
    let size = 3
    let label = if size > 2: "big"
    print(label)
""",
"""# Both branches give a value.
def main():
    let size = 3
    let label = if size > 2: "big" else: "small"
    print(label)
""",
"An `if` used as an expression has a value on every path, so it has an `else` branch.",
"Add the `else` branch with the value for the other case.")

entry("E0200", "e0200_misspelt",
"""# A misspelt name.
def main():
    prnt("Hei, maailma!")
""",
"""# The name as it is declared.
def main():
    print("Hei, maailma!")
""",
"A name is used that nothing in scope declares: no binding, parameter, item, import or prelude name. A binding is declared with `let` or `var`, not by assigning to a new name.",
"When a name in scope is spelt alike, the fix offers it. When the name is new, declare it with `let` (or `var`) before its first use.")

entry("E0202", "e0202_twice",
"""# Two functions with one name.
def area(width: int) -> int:
    return width * width

def area(width: int) -> int:
    return width * 2

def main():
    print(area(3))
""",
"""# Each function with a name of its own.
def square_area(width: int) -> int:
    return width * width

def strip_area(width: int) -> int:
    return width * 2

def main():
    print(square_area(3))
""",
"A module declares each name once. Sisu has no overloading (D-050), so two functions with one name are an error even when their parameters differ.",
"The note asks you to rename one or remove the one not wanted; a name that says what each computes is the usual way.")

entry("E0211", "e0211_ambiguous_variant",
"""# A variant name that two enums share.
enum Light:
    Red
    Green

enum Wine:
    Red
    White

def main():
    let colour = Red
    print("{colour:?}")
""",
"""# The variant named with its type.
enum Light:
    Red
    Green

enum Wine:
    Red
    White

def main():
    let colour = Light.Red
    print("{colour:?}")
""",
"A variant's own name is enough where only one enum has a variant of that name. Where several have, the bare name is ambiguous.",
"Qualify the variant with its type, as the message shows, or let a known type decide it: a parameter of type `Light` takes `Red`.")

entry("E0244", "e0244_derive_copy",
"""# `Copy` for a struct whose field is not `Copy`.
@derive(Copy)
struct Person:
    name: String

def main():
    let aino = Person("Aino")
    print(aino.name)
""",
"""# Without `Copy`; `.copy()` copies on request.
struct Person:
    name: String

def main():
    let aino = Person("Aino")
    print(aino.name)
""",
"A derive needs every field to have the trait it derives. A `String` is not `Copy`, since copying it takes new memory, which Sisu does only when asked.",
"Leave `Copy` out and copy with `.copy()` where a copy is wanted, or make the field a type that is `Copy`.")

entry("E0300", "e0300_mismatch",
"""# A text where a number is expected.
def main():
    let count: int = "3"
    print(count)
""",
"""# A number.
def main():
    let count: int = 3
    print(count)
""",
"A value of one type stands where another is expected. A label points at what made the other type expected.",
"The fix depends on the types; here it writes the number without its quotes. Where text must become a number, `parse_int()` converts it and says when it cannot.")

entry("E0301", "e0301_division",
"""# `/` between two integers.
def main():
    let pieces = 7
    print(pieces / 2)
""",
"""# `//` divides integers.
def main():
    let pieces = 7
    print(pieces // 2)
""",
"`/` divides floating-point numbers; integers are divided with `//`, which rounds towards negative infinity, and `%` gives the remainder.",
"The fix writes `//`. To divide with a fraction, convert first: `pieces.to_f64() / 2.0`.")

entry("E0302", "e0302_truthiness",
"""# A number used as a condition.
def main():
    let count = 3
    if count:
        print("some")
""",
"""# The test written out.
def main():
    let count = 3
    if count != 0:
        print("some")
""",
"A condition is a `bool`. Sisu has no truthiness: a number, a text or a collection is not true or false by itself (D-025).",
"Write the test you mean, such as `count != 0` or `not names.is_empty()`.")

entry("E0305", "e0305_argument_count",
"""# Two arguments to a function that takes one.
def main():
    print("Hei,", "maailma!")
""",
"""# One text, built by interpolation.
def main():
    let world = "maailma"
    print("Hei, {world}!")
""",
"A call passes as many arguments as the function has parameters, less those with defaults. `print` takes one text.",
"There is no fix to offer. For `print`, build one text with interpolation.")

entry("E0308", "e0308_missing_argument",
"""# A call without one of its arguments.
def area(width: int, height: int) -> int:
    return width * height

def main():
    print(area(3))
""",
"""# Every argument given.
def area(width: int, height: int) -> int:
    return width * height

def main():
    print(area(3, 4))
""",
"A parameter without a default needs an argument at every call.",
"The fix passes a typed hole, `todo(\"height\")`, which compiles and stops the program when it is reached; fill it with the value.")

entry("E0310", "e0310_operator",
"""# `+` between a text and a number.
def main():
    let age = 3
    print("age " + age)
""",
"""# The number interpolated into the text.
def main():
    let age = 3
    print("age {age}")
""",
"A binary operator is defined for some pairs of types only. `+` joins two texts, and adds two numbers of one type.",
"For a text and a value, the fix writes the value into the text by interpolation.")

entry("E0312", "e0312_assign_let",
"""# An assignment to a `let` binding.
def main():
    let count = 1
    count = 2
    print(count)
""",
"""# A `var` binding can be assigned.
def main():
    var count = 1
    count = 2
    print(count)
""",
"A `let` binding keeps its value; only a `var` binding, a `mut` parameter or a field of one can be assigned.",
"The fix declares the binding with `var`. For a parameter, it makes the parameter `mut`, which changes every call; a local `var` copy of it is often what you want instead.")

entry("E0314", "e0314_non_exhaustive",
"""# A `match` that misses a variant.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Green: "go"

def main():
    print(action(Amber))
""",
"""# An arm for every variant.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Amber: "wait"
        case Green: "go"

def main():
    print(action(Amber))
""",
"A `match` covers every value of its subject's type, so that adding a variant shows every `match` that must handle it.",
"Add an arm for the case the note names. A `case _` arm also covers it, and every variant added later.")

entry("E0315", "e0315_missing_return",
"""# A path that falls off the end of the body.
def sign(n: int) -> int:
    if n < 0:
        return -1
    elif n > 0:
        return 1

def main():
    print(sign(5))
""",
"""# A `return` on every path.
def sign(n: int) -> int:
    if n < 0:
        return -1
    elif n > 0:
        return 1
    return 0

def main():
    print(sign(5))
""",
"A function with a result type returns a value on every path; a value leaves a function only through `return` (D-056).",
"Return a value on the path that has none, often with a last `return` at the end of the body.")

entry("E0318", "e0318_unused_result",
"""# A result that nothing uses.
def main():
    "42".parse_int()
    print("done")
""",
"""# The result used.
def main():
    let n = "42".parse_int() else: 0
    print("done with {n}")
""",
"A value that can hold an error or an absence must be used, so that no failure passes unseen.",
"Handle it, with `?`, `else` or `match`, or discard it on purpose with `_ = ...`, which the fix writes.")

entry("E0320", "e0320_try_context",
"""# `?` in a function that returns nothing.
def main():
    let n = "42".parse_int()?
    print(n)
""",
"""# A fallback where there is no caller to return to.
def main():
    let n = "42".parse_int() else: 0
    print(n)
""",
"`?` returns early from the function with the absence or the error, so the function's result must be able to hold it: `T?` for an absence, `T ! E` for an error.",
"Give the function a result type that can hold it, as `def main() -> ! E` for an error, or handle the value where it is, with `else`.")

entry("E0325", "e0325_not_iterable",
"""# A `for` loop over a text.
def main():
    for letter in "hei":
        print(letter)
""",
"""# A loop over the text's characters.
def main():
    for letter in "hei".chars():
        print(letter)
""",
"A `for` loop takes a value that can be iterated: a collection, a range or an iterator. A text is not, because it can be read as characters, bytes or lines.",
"Say which: `.chars()`, `.bytes()` or `.lines()`.")

entry("E0327", "e0327_unknown_variant",
"""# A pattern that names a variant the enum does not have.
enum Light:
    Red
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Blue: "wait"
        case _: "go"

def main():
    print(action(Red))
""",
"""# Only the enum's own variants.
enum Light:
    Red
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case _: "go"

def main():
    print(action(Red))
""",
"A capitalised name in a pattern is a variant of the subject's type, and that type has none of that name.",
"Name a variant the type has, or add it to the enum. A lowercase name in a pattern binds the value instead.")

entry("E0329", "e0329_refutable_let",
"""# A `let` whose pattern might not match.
def main():
    let maybe: int? = Some(3)
    let Some(n) = maybe
    print(n)
""",
"""# `else` says what happens when it does not match.
def main():
    let maybe: int? = Some(3)
    let Some(n) = maybe else:
        return
    print(n)
""",
"The pattern of a plain `let` matches every value. A pattern that might not match needs `else:` with a block that leaves the scope.",
"Add `else:` with a `return`, `break`, `continue` or a panic, or use `if let` or `match`.")

entry("E0337", "e0337_missing_field",
"""# A struct built without one of its fields.
struct Point:
    x: int
    y: int

def main():
    let p = Point(x=1)
    print(p.x)
""",
"""# Every field given.
struct Point:
    x: int
    y: int

def main():
    let p = Point(x=1, y=2)
    print(p.x)
""",
"Building a struct gives a value to every field that has no default.",
"The fix passes a typed hole for the field; give the value, or give the field a default in the struct.")

entry("E0341", "e0341_not_mutable",
"""# A change to a `let` binding's list.
def main():
    let names = ["Aino"]
    names.push("Eero")
    print(names)
""",
"""# A `var` binding can change.
def main():
    var names = ["Aino"]
    names.push("Eero")
    print(names)
""",
"A method that changes its value (`mut self`), or a change to a field, needs a value that can change: a `var` binding, a `mut` parameter, or a `mut` borrow.",
"The fix declares the binding with `var`. For a method of your own that changes `self`, the first parameter is `mut self`.")

entry("E0344", "e0344_negative_index",
"""# Python's last element.
def main():
    let names = ["Aino", "Eero"]
    print(names[-1])
""",
"""# `^1` counts from the end.
def main():
    let names = ["Aino", "Eero"]
    print(names[^1])
""",
"Sisu counts from the end with `^`: `xs[^1]` is the last element. A constant negative index does not compile, so that Python's `xs[-1]` never means something else (D-062).",
"The fix writes `^1`; an index computed at run time that is negative panics instead.")

entry("E0345", "e0345_str_index",
"""# A text indexed by position.
def main():
    let word = "hei"
    print(word[0])
""",
"""# The first character, from the characters.
def main():
    let word = "hei"
    let first = word.chars().next() else: ' '
    print(first)
""",
"A `str` has no integer index, since its characters take different numbers of bytes (D-034).",
"Iterate its `.chars()` or `.bytes()`, or take a part with `.byte_slice(a, b)` at known character boundaries.")

entry("E0347", "e0347_bound",
"""# `==` on a struct that does not implement `Eq`.
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
""",
"""# `@derive(Eq)` gives the struct `==`.
@derive(Eq)
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
""",
"A type is used where a trait is required, and does not implement it. `==` needs `Eq`, sorting needs `Ord`, a set or a dict key needs `Hash`.",
"Derive the trait with `@derive(...)` on the type, or write an `impl` of it.")

entry("E0368", "e0368_not_ordered",
"""# `<` on a struct that has no order.
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) < Point(2, 1))
""",
"""# `@derive(Eq, Ord)` orders the struct field by field.
@derive(Eq, Ord)
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) < Point(2, 1))
""",
"`<`, `<=`, `>`, `>=`, `sorted`, `min` and `max` need an order, the trait `Ord`.",
"Derive `Ord` (with `Eq`), which compares the fields in the order they are declared, or write an `impl`.")

entry("E0373", "e0373_mixed_numbers",
"""# An integer and a float in one operation.
def main():
    let price = 3
    print(price * 1.5)
""",
"""# The integer converted first.
def main():
    let price = 3
    print(price.to_f64() * 1.5)
""",
"An arithmetic operator takes two numbers of one type; Sisu converts between `int` and `f64` only when you say so.",
"The fix converts the integer with `.to_f64()`; to go the other way, `round()` or `truncate_to_int()` says how.")

entry("E0374", "e0374_out_of_range",
"""# A literal too large for its type.
def main():
    let level: u8 = 300
    print(level)
""",
"""# A value the type holds.
def main():
    let level: u8 = 250
    print(level)
""",
"An integer literal fits in the type it gets. The note gives the type's range.",
"Use a value in the range, or a wider type such as `int`.")

entry("E0400", "e0400_moved",
"""# A list used after it moved.
def main():
    let names = ["Aino"]
    let team = names
    print(names)
    print(team)
""",
"""# A copy for the second owner.
def main():
    let names = ["Aino"]
    let team = names.copy()
    print(names)
    print(team)
""",
"A value has one owner. Assigning a value that is not `Copy` moves it, and the old name can no longer be used.",
"The note offers two ways: copy the value before the move, which costs a copy, or use the old name before the move. Reading through a borrow instead of moving is the third.")

entry("E0411", "e0411_view_field",
"""# A borrowed text in a plain struct.
struct Order:
    item: str

def main():
    let order = Order("bread")
    print(order.item)
""",
"""# The struct owns its text.
struct Order:
    item: String

def main():
    let order = Order("bread")
    print(order.item)
""",
"A plain struct owns all its fields; a field that borrows, such as `str`, could outlive what it borrows. Only a `view struct` may hold a borrow.",
"The fix makes the struct a `view struct`, which limits where its values can go. Usually the field should own its value: `String` for `str`.")

entry("W0001", "w0001_hole",
"""# A typed hole in a function's body.
def double(x: int) -> int:
    return todo("double")

def main():
    print("ready")
""",
"""# The hole filled.
def double(x: int) -> int:
    return x * 2

def main():
    print("ready, {double(21)}")
""",
"A typed hole, `todo(\"name\")`, stands for code still to be written (D-061). The program compiles and runs until it reaches the hole, which panics; `sisu test` reports a test that reaches one as blocked.",
"Write the code the hole stands for.", kind="warning")

entry("W0002", "w0002_shadow",
"""# An inner binding that hides an outer one.
def main():
    let total = 1
    if total > 0:
        let total = 2
        print(total)
    print(total)
""",
"""# Each binding with a name of its own.
def main():
    let total = 1
    if total > 0:
        let doubled = 2
        print(doubled)
    print(total)
""",
"A binding in an inner block with the name of a binding of an enclosing block hides it until the inner block ends (D-024). Rebinding in the same block does not warn.",
"Give the inner binding a name of its own, or assign to the outer one if it is a `var`.", kind="warning")

entry("W0003", "w0003_unreachable",
"""# An arm that the arm above it covers.
def describe(n: int) -> str:
    return match n:
        case _: "a number"
        case 0: "zero"

def main():
    print(describe(0))
""",
"""# The specific arm first.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case _: "a number"

def main():
    print(describe(0))
""",
"Arms are tried from the top, so an arm whose values the arms above it all cover never runs.",
"Move the specific arm above the general one, or remove it.", kind="warning")

# Run-time messages: the wrong program fails when it runs.
entry("R0001", "r0001_panic",
"""# A panic for a case the program cannot handle.
def weekday(n: int) -> str:
    if n < 1 or n > 7:
        panic("no weekday {n}")
    return ["ma", "ti", "ke", "to", "pe", "la", "su"][n - 1]

def main():
    print(weekday(3))
    print(weekday(8))
""",
"""# An absent value for a number that names no weekday.
def weekday(n: int) -> str?:
    if n < 1 or n > 7:
        return None
    return Some(["ma", "ti", "ke", "to", "pe", "la", "su"][n - 1])

def main():
    print(weekday(3) ?? "none")
    print(weekday(8) ?? "none")
""",
"`panic(message)` stops the task with the message, and so does `expect(message)` on an absent value. A panic is for a bug, a state the program should never reach; an input that can be wrong is an error or an absent value instead.",
"When the case can happen, return `T?` or `T ! E` and let the caller decide.", kind="runtime")

entry("R0002", "r0002_index",
"""# An index past the end of a list.
def main():
    let scores = [3, 1, 4]
    print(scores[3])
""",
"""# `.get` gives an absent value past the end.
def main():
    let scores = [3, 1, 4]
    print(scores.get(3) ?? 0)
""",
"An index is checked against the length when it is used, and an index past the end panics, naming both.",
"Check the index first, or use `.get(i)`, which gives `None` past the end.", kind="runtime")

entry("R0003", "r0003_overflow",
"""# An addition past the largest `int`.
def main():
    let big = 9_223_372_036_854_775_807
    print(big + 1)
""",
"""# `+%` wraps around on purpose.
def main():
    let big = 9_223_372_036_854_775_807
    print(big +% 1)
""",
"Integer arithmetic that leaves its type's range panics, naming the operation and its operands (D-009).",
"Use a wider type, or say what should happen with the wrapping (`+%`) or saturating (`+|`) operators.", kind="runtime")

entry("R0004", "r0004_division",
"""# A division by zero.
def share(total: int, people: int) -> int:
    return total // people

def main():
    print(share(10, 0))
""",
"""# The zero handled before dividing.
def share(total: int, people: int) -> int:
    if people == 0:
        return 0
    return total // people

def main():
    print(share(10, 0))
""",
"An integer division or remainder by zero panics.",
"Check the divisor before dividing, or state it as a precondition with `requires`.", kind="runtime")

entry("R0006", "r0006_assert",
"""# An assertion that does not hold.
def main():
    let names = ["Aino", "Eero"]
    assert len(names) == 3
    print("three names")
""",
"""# An assertion that holds.
def main():
    let names = ["Aino", "Eero", "Roni"]
    assert len(names) == 3
    print("three names")
""",
"`assert` checks a condition and panics when it is false, showing the value of each part of the expression.",
"Make the condition hold, or correct the assertion. In a test, a failed `assert` fails the test.", kind="runtime")

entry("R0010", "r0010_missing_key",
"""# A key that the dict does not hold.
def main():
    let ages = {"Aino": 7, "Eero": 9}
    print(ages["Roni"])
""",
"""# `.get` gives an absent value for a missing key.
def main():
    let ages = {"Aino": 7, "Eero": 9}
    print(ages.get("Roni") ?? 0)
""",
"Indexing a dict with a key it does not hold panics, naming the key.",
"Look the key up with `.get(key)`, which gives `None`, or test `key in ages` first.", kind="runtime")

entry("R0012", "r0012_zip",
"""# `zip` over two lists of different lengths.
def main():
    for name, age in zip(["Aino", "Eero"], [7]):
        print("{name} {age}")
""",
"""# `zip_shortest` stops at the shorter list.
def main():
    for name, age in zip_shortest(["Aino", "Eero"], [7]):
        print("{name} {age}")
""",
"`zip` expects its arguments to be equally long, and panics when one runs out first.",
"Make them equally long, or use `zip_shortest` when stopping at the shorter is what you want.", kind="runtime")

entry("R0013", "r0013_hole",
"""# A run that reaches a typed hole.
def double(x: int) -> int:
    return todo("double")

def main():
    print(double(21))
""",
"""# The hole filled.
def double(x: int) -> int:
    return x * 2

def main():
    print(double(21))
""",
"A run that reaches a typed hole panics with the hole's name; the compiler warned about it with W0001.",
"Write the code the hole stands for.", kind="runtime")

entry("R0018", "r0018_main_error",
"""# `main` returns the error of a failed parse.
error InputError:
    NotANumber(text: String)

def number(text: str) -> int ! InputError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def main() -> ! InputError:
    print(number("kolme")?)
    return Ok(())
""",
"""# The error handled where it happens.
error InputError:
    NotANumber(text: String)

def number(text: str) -> int ! InputError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def main():
    match number("kolme"):
        case Ok(n): print(n)
        case Err(e): print("not a number: {e:?}")
""",
"An error that `main` returns ends the program: it is printed with the place that passed it on, and the exit status is 1.",
"Return the error when ending the program is right; handle it with `match`, `else` or `.or` when the program can go on.", kind="runtime")

entry("R0100", "r0100_precondition",
"""# A call that breaks its function's precondition.
def average(total: int, count: int) -> int:
    requires count > 0
    return total // count

def main():
    print(average(10, 0))
""",
"""# The call checks what the function requires.
def average(total: int, count: int) -> int:
    requires count > 0
    return total // count

def main():
    let count = 0
    if count > 0:
        print(average(10, count))
    else:
        print("nothing to average")
""",
"A `requires` clause states what a function needs of its caller; a call that breaks it panics with the clause and the values of its parts.",
"The caller is wrong: check the condition before the call.", kind="runtime")

entry("R0101", "r0101_postcondition",
"""# A body that breaks its own postcondition.
def rounded_up(amount: int, step: int) -> int:
    requires step > 0
    ensures result >= amount
    return amount // step * step

def main():
    print(rounded_up(7, 5))
""",
"""# A body that keeps its promise.
def rounded_up(amount: int, step: int) -> int:
    requires step > 0
    ensures result >= amount
    return (amount + step - 1) // step * step

def main():
    print(rounded_up(7, 5))
""",
"An `ensures` clause states what a function promises; a return that breaks it panics with the clause and the values of its parts.",
"The function is wrong: correct its body, or its promise.", kind="runtime")

entry("R0115", "r0115_char_boundary",
"""# A byte slice that cuts a character in two.
def main():
    let word = "äiti"
    print(word.byte_slice(0, 1))
""",
"""# A slice that ends at a character boundary.
def main():
    let word = "äiti"
    print(word.byte_slice(0, 2))
""",
"A text is its UTF-8 bytes, and `ä` takes two of them. A byte index inside a character panics.",
"Slice at character boundaries, or iterate `.chars()`.", kind="runtime")

# Codes that R1 and R19 show.
entry("E0002", "e0002_dedent",
"""# A line that matches no enclosing block's indentation.
def main():
    let n = 2
    if n > 1:
        print("big")
      print("done")
""",
"""# The line back at the indentation of its block.
def main():
    let n = 2
    if n > 1:
        print("big")
    print("done")
""",
"A line that is indented less than the one before closes blocks until its indentation matches an enclosing block's. When it matches none, the message names the columns that would.",
"Indent the line to one of the columns the message names: the block it belongs to.")

entry("E0011", "e0011_semicolon",
"""# Two statements on one line.
def main():
    let a = 1; let b = 2
    print(a + b)
""",
"""# One statement per line.
def main():
    let a = 1
    let b = 2
    print(a + b)
""",
"A statement takes one line, and a line break ends it; Sisu has no semicolons (D-020).",
"Put each statement on its own line.")

entry("E0012", "e0012_number",
"""# A number with letters that are not a suffix.
def main():
    print(12abc)
""",
"""# A number, and a suffix that names a type.
def main():
    print(12u8)
""",
"A number literal is digits, with `_` between them, a prefix `0x`, `0b` or `0o`, and a suffix that names a numeric type ([R1](lexical-structure.md#integer-literals)).",
"Write the number without the letters, or with a suffix such as `u8` or `f32`.")

entry("E0015", "e0015_backslash",
"""# A backslash at the end of a line.
def main():
    let total = 1 + \\
        2
    print(total)
""",
"""# The expression on one line, as `sisu fmt` writes it.
def main():
    let total = 1 + 2
    print(total)
""",
"A logical line continues inside brackets, after a comma or an open bracket, and before a binary operator; a backslash does not continue it (D-020).",
"Join the lines, or break the line in one of the three ways the message names; `sisu fmt` joins it again when it fits in 100 columns.")

entry("E0102", "e0102_same_line",
"""# A block on the line that opens it.
def main():
    for n in [1, 2]: print(n)
""",
"""# The block on the next line, indented.
def main():
    for n in [1, 2]:
        print(n)
""",
"A block starts on the line after the `:` that opens it, indented (D-027).",
"Move what follows the `:` to the next line and indent it.")

entry("E0107", "e0107_top_level",
"""# A statement outside every function.
print("Hei")

def main():
    print("maailma")
""",
"""# The statement inside `main`.
def main():
    print("Hei")
    print("maailma")
""",
"A module holds declarations; statements belong in functions. A script, a file that starts with `#!`, is the exception: its top-level statements are its `main`. Python's `class` is reported here too, since it is not a declaration.",
"Move the statement into `main` or another function, or make the file a script.")

entry("E0117", "e0117_trailing",
"""# Two statements on one line, without a semicolon.
def main():
    let n = 3 print(n)
""",
"""# One statement per line.
def main():
    let n = 3
    print(n)
""",
"After a statement ends, only a line break may follow. The parser reports the first token it did not expect, which often stands for a habit from another language: a Rust comment `//`, `lambda`, `println!`.",
"End the statement where it ends, and put what follows on a line of its own, in Sisu's spelling.")

entry("E0124", "e0124_keyword",
"""# A keyword used as a parameter's name.
def score(match: str) -> int:
    return 0

def main():
    print(score("final"))
""",
"""# A parameter named with a name.
def score(game: str) -> int:
    return len(game)

def main():
    print(score("final"))
""",
"A keyword is not a name, except after `.`, `?.` and `def` ([R1](lexical-structure.md#keywords)).",
"Choose another name. A keyword elsewhere where a name should be gets the parser's E0100, E0103 or E0105 instead.")

entry("E0215", "e0215_self",
"""# A `self` parameter in a function that is not a method.
def describe(self) -> String:
    return "a thing"

def main():
    print(describe())
""",
"""# A function without `self`.
def describe() -> String:
    return "a thing"

def main():
    print(describe())
""",
"`self` is the first parameter of a method, in the body of a `struct`, `enum`, `trait` or `impl`; as a parameter of any other function it is E0215, and as a value outside a method E0389.",
"Leave `self` out of a free function, or move the function into the type's body as a method.")

entry("E0389", "e0389_self",
"""# `self` used in a function that is not a method.
def describe() -> String:
    return "{self}"

def main():
    print(describe())
""",
"""# The value passed as a parameter.
def describe(name: str) -> String:
    return "{name}"

def main():
    print(describe("Aino"))
""",
"`self` names the value a method is called on, so it exists only inside a method.",
"Pass the value as a parameter, or move the function into the type's body as a method.")

entry("E0304", "e0304_method",
"""# A method that `str` does not have.
def main():
    print("  hei  ".strip())
""",
"""# `trim` removes the spaces at both ends.
def main():
    print("  hei  ".trim())
""",
"A method call names a method of the value's type or of a trait it implements. A note names the trait when another type has the method, as `join` of `Iterator`.",
"Use the type's method; [R19](false-friends.md) lists the Python and Rust names that differ.")

entry("E0306", "e0306_keyword_argument",
"""# A keyword argument that the function does not have.
def greet(name: str, polite: bool = False) -> String:
    return if polite: "Good day, {name}" else: "Hei, {name}"

def main():
    print(greet("Aino", formal=True))
""",
"""# The parameter's own name.
def greet(name: str, polite: bool = False) -> String:
    return if polite: "Good day, {name}" else: "Hei, {name}"

def main():
    print(greet("Aino", polite=True))
""",
"A keyword argument names a parameter of the function.",
"Use the parameter's name as the function declares it.")

entry("E0406", "e0406_short_life",
"""# A closure that borrows the loop's `i` outlives the iteration.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(() -> i)
    print(len(steps))
""",
"""# An `own` closure takes its own copy of `i`.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(own () -> i)
    print(len(steps))
""",
"A borrow cannot be used after what it borrows is gone. The message names the line of the borrow and the line of its last use.",
"Declare the borrowed value where it lives long enough, as the note says, or move a copy into what outlives it: an `own` closure, or `.copy()`.")

# Codes that chapters 10 to 15 show.
entry("E0338", "e0338_move_out_of_borrow",
"""# An element moved out of a list that is borrowed.
def main():
    let queue: list[String] = ["Aino", "Eero"]
    let first = queue[0]
    print("{first} is first of {len(queue)}")
""",
"""# A copy of the element, which the list keeps.
def main():
    let queue: list[String] = ["Aino", "Eero"]
    let first = queue[0].copy()
    print("{first} is first of {len(queue)}")
""",
"Indexing reads through a borrow of the collection, so it cannot move an element that is not `Copy` out of it: the collection would be left with a hole.",
"The fix copies the element, which costs a copy. To take it out instead, use a method that removes it, such as `pop` or `remove`, on a `var` collection.")

entry("E0401", "e0401_partial_move",
"""# A struct used whole after a field moved out of it.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    let items = order.items
    print("{len(items)} items")
    print("{order:?}")
""",
"""# The struct used before the field moves out.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    print("{order:?}")
    let items = order.items
    print("{len(items)} items")
""",
"Moving a field out of a struct leaves the struct without it: the other fields can still be used, and the struct as a whole cannot.",
"Use the whole struct before the move, as the note says, or copy the field with `.copy()` and leave the struct whole.")

entry("E0339", "e0339_mut_at_call",
"""# A `mut` parameter called without `mut` at the call.
def deposit(mut balance: int, amount: int):
    balance += amount

def main():
    var balance = 100
    deposit(balance, 50)
    print(balance)
""",
"""# The call says `mut` too.
def deposit(mut balance: int, amount: int):
    balance += amount

def main():
    var balance = 100
    deposit(mut balance, 50)
    print(balance)
""",
"A call passes an argument to a `mut` parameter with `mut` before it, so that the reader of the call sees what can change (D-006).",
"The fix writes `mut` before the argument.")

entry("E0402", "e0402_modify_while_borrowed",
"""# A list changed while a borrow of an element is still used.
def main():
    var names: list[String] = ["Aino", "Eero", "Roni"]
    let first = ref names[0]
    names[1] = "Ella"
    print("{first} is first")
""",
"""# The change after the borrow's last use.
def main():
    var names: list[String] = ["Aino", "Eero", "Roni"]
    let first = ref names[0]
    print("{first} is first")
    names[1] = "Ella"
""",
"A value that is borrowed cannot change until the borrow's last use. A borrow ends at its last use, not at the end of its block.",
"Move the change after the line the note names, or work on a copy.")

entry("E0405", "e0405_mut_while_borrowed",
"""# A method that changes a list called while it is borrowed.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    scores.push(10)
    print("first {first}, of {len(scores)}")
""",
"""# The borrow's last use before the change.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    print("first {first}")
    scores.push(10)
    print("{len(scores)} scores")
""",
"A value has either one `mut` borrow or any number of shared ones at a time. A call of a `mut self` method borrows with `mut`.",
"Finish with the shared borrow first, as the note says, or copy what it reads.")

entry("E0407", "e0407_return_local",
"""# A function that returns a borrow of its own local.
def greeting(name: str) -> str:
    let text = "Hei, {name}!"
    return text

def main():
    print(greeting("Aino"))
""",
"""# The function returns the owned text.
def greeting(name: str) -> String:
    let text = "Hei, {name}!"
    return text

def main():
    print(greeting("Aino"))
""",
"A function's locals are gone when it returns, so it cannot return a borrow of one; `str` is a borrow, and an interpolated literal is a local `String`.",
"Return an owned value: here, make the result type `String`.")

entry("E0413", "e0413_from",
"""# A returned borrow that could come from either parameter.
def longer(a: str, b: str) -> str:
    return if len(a) >= len(b): a else: b

def main():
    print(longer("sauna", "sisu"))
""",
"""# `from` names where the borrow comes from.
def longer(a: str, b: str) -> str from a, b:
    return if len(a) >= len(b): a else: b

def main():
    print(longer("sauna", "sisu"))
""",
"A function that returns a borrow and has one borrowed parameter returns a borrow of it. With more than one, `from` names those the result may borrow from, so a caller knows what must outlive the result.",
"The fix names every borrowed parameter; name fewer when the result borrows from fewer.")

entry("E0417", "e0417_must_consume",
"""# A must-consume value that goes out of scope unconsumed.
@must_consume
struct Draft:
    text: String

    def discard(own self):
        pass

def main():
    let note = Draft("a first try")
    print("drafted {note.text}")
""",
"""# The value consumed by a method that takes `own self`.
@must_consume
struct Draft:
    text: String

    def discard(own self):
        pass

def main():
    let note = Draft("a first try")
    print("drafted {note.text}")
    note.discard()
""",
"A `@must_consume` type cannot be dropped silently: each value is passed to an `own` parameter before it goes out of scope (D-054).",
"Consume it with one of the methods the note names, each of which says what ending it means.")

entry("E0231", "e0231_trait_as_type",
"""# A trait used where a type should be.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: Priced) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
""",
"""# A type parameter bounded by the trait.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show[T: Priced](item: T) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
""",
"A trait is not a type. A value of some type that implements it is a type parameter with the trait as its bound, or a `dyn` value behind `ref` or `Box`.",
"The fix writes `dyn Priced`, which then needs `ref` or `Box` (E0395); a bounded type parameter is the usual choice.")

entry("E0232", "e0232_missing_method",
"""# An impl without one of the trait's required methods.
trait Priced:
    def price(self) -> int
    def name(self) -> String

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def main():
    print(Bun("cardamom").price())
""",
"""# Every required method written.
trait Priced:
    def price(self) -> int
    def name(self) -> String

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

    def name(self) -> String:
        return "a {self.filling} bun"

def main():
    print(Bun("cardamom").price())
""",
"An `impl` of a trait writes every method the trait declares without a body.",
"Write the method the note names, with the signature it gives.")

entry("E0235", "e0235_overlap",
"""# Two impls of one trait for one type.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("€{self.cents // 100}")

def main():
    print(Money(1250))
""",
"""# One impl.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

def main():
    print(Money(1250))
""",
"A type implements a trait at most once, so that a call never has two meanings (D-013).",
"Keep one impl. For a second way to show the value, write a method, or wrap the value in a type of its own.")

entry("E0242", "e0242_supertrait",
"""# An impl of a trait whose supertrait the type lacks.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Weight:
    grams: int

impl Labelled for Weight:
    pass

def main():
    print(Weight(90).label())
""",
"""# The supertrait implemented too.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Weight:
    grams: int

impl Display for Weight:
    def fmt(self, mut f: Formatter):
        f.write("{self.grams} g")

impl Labelled for Weight:
    pass

def main():
    print(Weight(90).label())
""",
"A trait that names a supertrait, as `trait Labelled(Display)`, can be implemented only for types that implement the supertrait.",
"Implement the supertrait for the type as well.")

entry("E0243", "e0243_not_derivable",
"""# A derive of a trait that has no derive.
@derive(Debug, Display)
struct Money:
    cents: int

def main():
    print("{Money(1250):?}")
""",
"""# `Display` written by hand.
@derive(Debug)
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

def main():
    print(Money(1250))
""",
"`@derive` takes `Copy` and the traits that declare a `comptime def derive`; the message lists them. `Display` has none, since only you know how your type should read.",
"Write an `impl` of the trait.")

entry("E0241", "e0241_object_safety",
"""# A trait used as `dyn` with a method that takes no `self`.
trait Shape:
    def unit() -> Self
    def area(self) -> f64

struct Square:
    side: f64

impl Shape for Square:
    def unit() -> Square:
        return Square(1.0)

    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    return sum([shape.area() for shape in shapes])

def main():
    print(total([Box(Square(2.0))]))
""",
"""# The method without `self` moved out of the trait.
trait Shape:
    def area(self) -> f64

struct Square:
    side: f64

    def unit() -> Square:
        return Square(1.0)

impl Shape for Square:
    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    return sum([shape.area() for shape in shapes])

def main():
    print(total([Box(Square(2.0)), Box(Square.unit())]))
""",
"A trait can be a `dyn` type only when every method can be called through it: each takes `self` or `mut self`, and none names `Self` elsewhere (D-017).",
"Move the method out of the trait, into the type or another trait, or use a type parameter instead of `dyn`.")

entry("E0395", "e0395_bare_dyn",
"""# A `dyn` value passed by itself.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: dyn Priced) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
""",
"""# The `dyn` value behind `ref`.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: ref dyn Priced) -> String:
    return "{item.price()} cents"

def main():
    let bun = Bun("cardamom")
    print(show(ref bun))
""",
"A `dyn Trait` value can be of any size, since any type that implements the trait can stand there, so it is held behind `ref` or in a `Box`.",
"Hold it behind `ref`, or in a `Box` to own it.")

# Codes that R6 shows.
entry("E0328", "e0328_field_count",
"""# A struct pattern that leaves out a field without `..`.
struct Point:
    x: int
    y: int

def describe(p: Point) -> str:
    return match p:
        case Point(x=0): "on the y axis"
        case _: "elsewhere"

def main():
    print(describe(Point(0, 4)))
""",
"""# `..` stands for the fields the pattern does not name.
struct Point:
    x: int
    y: int

def describe(p: Point) -> str:
    return match p:
        case Point(x=0, ..): "on the y axis"
        case _: "elsewhere"

def main():
    print(describe(Point(0, 4)))
""",
"A struct or variant pattern gives a pattern for every field, by position or by keyword, unless it ends with `..`, which stands for the fields it leaves out.",
"Add `..` at the end of the pattern, or a pattern for each field; `_` for a field matches any value.")

entry("E0385", "e0385_or_bindings",
"""# An alternative that binds a name the other does not.
enum Reading:
    Celsius(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t) | Missing: t

def main():
    print(value(Missing))
""",
"""# Each alternative in an arm of its own.
enum Reading:
    Celsius(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t): t
        case Missing: 0.0

def main():
    print(value(Missing))
""",
"The alternatives of `|` run the same arm, so every alternative binds the same names, with the same types.",
"Bind the name in every alternative, or give the alternatives arms of their own.")
