use crate::{format, format_with};

fn fmt(s: &str) -> String {
    match format(s) {
        Ok(t) => t,
        Err(e) => panic!("refused: {e:?}"),
    }
}

#[test]
fn spacing_and_indentation() {
    let src = "def f(a:int,b:str='x')->int:\n  let d={'a':1}\n  return f( a=1 )+-a\n";
    let src = src.replace('\'', "\"");
    assert_eq!(
        fmt(&src),
        "def f(a: int, b: str = \"x\") -> int:\n    let d = {\"a\": 1}\n    return f(a=1) + -a\n"
    );
}

#[test]
fn idempotent_on_its_output() {
    let src = "import std.json\nfrom std.time import Duration, DateTime\nimport cli\n\n\n\nstruct P:\n    x: int\n    def get(self) -> int:\n        return self.x\ndef main():\n    print(json, cli, Duration, DateTime)\n";
    let once = fmt(src);
    assert_eq!(fmt(&once), once);
    assert!(
        once.starts_with("import cli\nimport std.json\nfrom std.time import DateTime, Duration\n\nstruct P:"),
        "{once}"
    );
}

#[test]
fn unused_imports_are_removed() {
    let src = "import std.json\nimport std.num as n\nfrom std.time import Duration, DateTime, Instant\npub from std.collections import Deque\n\n## `Instant` in a doc comment counts.\ndef main():\n    print(Duration.seconds(1))\n";
    let out = fmt(src);
    assert_eq!(
        out,
        "from std.time import Duration, Instant\npub from std.collections import Deque\n\n## `Instant` in a doc comment counts.\ndef main():\n    print(Duration.seconds(1))\n"
    );
    // One with a comment inside stays.
    let kept = "import std.json  # for later\n";
    assert_eq!(fmt(kept), kept);
}

#[test]
fn a_comment_at_the_end_does_not_break_the_line() {
    let src = format!("def main():\n    let x = f(a, b) + 1  # {}\n", "c".repeat(100));
    assert_eq!(fmt(&src), src);
}

#[test]
fn a_definition_breaks_its_parameters_first() {
    let src = format!(
        "def f[K](alpha: int, beta: int, gamma: (int) -> K) -> dict[K, {}]:\n    pass\n",
        "V".repeat(40)
    );
    let out = fmt(&src);
    assert_eq!(
        out,
        format!(
            "def f[K](\n    alpha: int, beta: int, gamma: (int) -> K\n) -> dict[K, {}]:\n    pass\n",
            "V".repeat(40)
        )
    );
}

#[test]
fn a_comprehension_breaks_before_its_clauses() {
    let long = "x".repeat(30);
    let src = format!(
        "def main():\n    let ys = [{long} * 2 for {long} in range(10) if {long} > 2 and {long} < 8]\n"
    );
    let out = fmt(&src);
    assert_eq!(
        out,
        format!(
            "def main():\n    let ys = [\n        {long} * 2\n        for {long} in range(10)\n        if {long} > 2 and {long} < 8\n    ]\n"
        )
    );
}

#[test]
fn prefixes_and_supertraits_hug() {
    assert_eq!(fmt("def f(x: int) -> int:\n    return ~ (x)\n"), "def f(x: int) -> int:\n    return ~(x)\n");
    assert_eq!(fmt("trait Ord (Eq):\n    pass\n"), "trait Ord(Eq):\n    pass\n");
}

#[test]
fn long_calls_break_at_brackets() {
    let src = format!(
        "def main():\n    print(some_function({}, {}, {}))\n",
        "a".repeat(40),
        "b".repeat(40),
        "c".repeat(40)
    );
    let out = fmt(&src);
    assert!(out.contains("    print(\n        some_function(\n"), "{out}");
    assert_eq!(fmt(&out), out);
}

#[test]
fn comments_are_kept() {
    let src = "# head\n\ndef main():  # trailing\n    # inside\n    let x = [\n        1,  # one\n        # alone\n        2,\n    ]\n    # end of body\n# after\n";
    let out = fmt(src);
    assert_eq!(out, src);
}

#[test]
fn option_sugar() {
    assert_eq!(
        fmt("def f(x: Option[int]) -> Option[str]:\n    pass\n"),
        "def f(x: int?) -> str?:\n    pass\n"
    );
    // Not when it would read as `??`.
    assert_eq!(fmt("def f(x: Option[int?]):\n    pass\n"), "def f(x: Option[int?]):\n    pass\n");
}

#[test]
fn result_sugar_for_error_sets() {
    let sets = |t: &str| t == "E" || t == "io.Error";
    let src = "def f(x: Result[int, E], y: Result[int, T]) -> Result[str, io.Error]:\n    pass\n";
    assert_eq!(
        format_with(src, &sets).as_deref(),
        Ok("def f(x: int ! E, y: Result[int, T]) -> str ! io.Error:\n    pass\n")
    );
    // Not without the names, not behind `ref` or `?`, and not when the
    // value is a function or a result itself.
    assert_eq!(format(src).as_deref(), Ok(src));
    for kept in [
        "def f(x: ref Result[int, E]):\n    pass\n",
        "def f(x: Result[(int) -> int, E]):\n    pass\n",
        "def f(x: Result[int, E]?):\n    pass\n",
    ] {
        assert_eq!(format_with(kept, &sets).as_deref(), Ok(kept));
    }
    assert_eq!(
        format_with("def f(x: Result[Result[int, E], E]):\n    pass\n", &sets).as_deref(),
        Ok("def f(x: Result[int ! E, E]):\n    pass\n")
    );
    // An optional result keeps `Option`, since `int ! E?` would make the
    // error optional.
    assert_eq!(
        format_with("def f(x: Option[Result[int, E]]):\n    pass\n", &sets).as_deref(),
        Ok("def f(x: Option[int ! E]):\n    pass\n")
    );
}

#[test]
fn refuses_broken_code() {
    assert!(format("def f(:\n").is_err());
}
