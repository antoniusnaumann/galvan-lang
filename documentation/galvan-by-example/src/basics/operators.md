# Operators

Galvan's operators mostly follow convention, with a few deliberate changes:
exponentiation is `^`, bitwise xor is `~`, and the word operators `and`, `or`,
`xor` are first-class:

```galvan
fn main() {
    let sum = 1 + 2 * 3
    let grouped = (1 + 2) * 3
    let remainder = 7 % 3
    let power: U32 = 2 ^ 10

    let both = true and false
    let either = true or false
    let one_of = true xor false

    let bits_or = 5 | 3
    let bits_and = 6 & 3
    let bits_xor = 5 ~ 3
    let shifted = 1 << 4

    let close = 1 ≤ 2 // or 1 <= 2 
    let far   = 3 ≥ 4 // or 3 >= 4
    let uneq  = 5 ≠ 6 // or 5 != 6
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let sum: _ = 1 + 2 * 3;
    let grouped: _ = (1 + 2) * 3;
    let remainder: _ = 7 % 3;
    let power: u32 = 2.pow(10);
    let both: bool = true && false;
    let either: bool = true || false;
    let one_of: bool = true ^ false;
    let bits_or: _ = 5 | 3;
    let bits_and: _ = 6 & 3;
    let bits_xor: _ = 5 ^ 3;
    let shifted: _ = 1 << 4;
    let close: bool = (1).le(&2);
    let far: bool = (3).ge(&4);
    let different: bool = (5).ne(&6);
}
```

Exponentiation lowers to `.pow(..)`; both logical `xor` and bitwise `~` become
Rust's `^` — the Galvan spelling difference exists so that logical and bitwise
intent stay visually distinct in source code. Comparisons lower to the
`PartialOrd`/`PartialEq` method forms so that borrowed operands compare
without explicit dereferencing.

</details>

- **Arithmetic**: `+`, `-`, `*`, `/`, `%`, and `^` for exponentiation.
- **Logical**: `and`/`&&`, `or`/`||`, `xor`.
- **Bitwise**: `|`, `&`, `~` (xor), `<<`, `>>`.
- **Comparison**: `==`, `!=`/`≠`, `<`, `<=`/`≤`, `>`, `>=`/`≥`. The Unicode
  spellings are interchangeable with their ASCII forms.
- **Identity**: `===`/`≡` and `!==`/`≢` compare *pointer identity* of heap
  references (see [Reference Variables](../ownership/ref_variables.md)).
- **Collections**: `++` (concatenation), `in`/`∈` (membership), `[]`
  (indexing) — covered in [Collection Operators](../collections/operators.md).
- **Ranges**: `..<`, `..=`, `..+`, `±` — covered in
  [Ranges](../control/ranges.md).

> [!WARNING]
> **Not implemented yet:** unary logical `not`, collection removal `--`,
> repetition `**`, slicing `[:]`, and user-defined custom operators are part
> of the operator design but do not transpile yet.
