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

The same rule powers struct declarations: fields separated by newlines get
commas inferred, so both of these are valid:

```galvan
type Point {
    x: Double
    y: Double
}

type Vector { x: Double, y: Double }
```

Explicit semicolons remain legal for putting several statements on one line:

```galvan
fn main() {
    let a = 1; let b = 2
    println("\(a + b)")
}
```
