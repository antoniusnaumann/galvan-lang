# Semicolons and Newlines

Galvan separates statements with semicolons but infers them at newlines, so
you almost never write one. A semicolon is inferred when the next line starts
with:

- an alphabetic character or underscore, or
- one of `{`, `(`, `[`, `'`, `"`.

Crucially, Galvan does **not** infer a semicolon when the current line is not
yet a valid statement. That makes multi-line expressions work without
continuation characters:

```galvan
fn main() {
    let total = 1 +
        2 +
        3

    let quantities = [1, 2, 3, 4]
    let doubled = quantities
        .iter()
        .copied()
        .map |it| { it * 2 }
        .vec()
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let total: _ = 1 + 2 + 3;
    let quantities: ::std::vec::Vec<_> = vec![1, 2, 3, 4];
    let doubled: _ = quantities.iter().copied().map(|it| it * 2).vec();
}
```

</details>

The same rule powers struct declarations: fields separated by newlines get
commas inferred, so both of these are valid:

```galvan
type Point {
    x: Double
    y: Double
}

type Vector { x: Double, y: Double }
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Vector {
    pub(crate) x: f64,
    pub(crate) y: f64,
}
```

</details>

The language design also permits explicit semicolons for putting several
statements on one line:

```galvan
fn main() {
    let a = 1; let b = 2
    println("\(a + b)")
}
```

> [!WARNING]
> Explicit same-line separators are not accepted by the parser yet. Newline
> inference and multiline expressions are implemented and checked above.
