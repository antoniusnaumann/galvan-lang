# Ranges

Galvan makes the upper bound explicit: `..<` excludes it, `..=` includes it.
Two more range operators cover common numeric patterns — `a ..+ n` starts at
`a` and spans `n` values, and `m ± d` is the inclusive tolerance range around
`m`:

```galvan
fn main() {
    mut sum = 0

    for 0..<5 |i| {   // 0, 1, 2, 3, 4
        sum += i
    }

    for 1..=3 {       // 1, 2, 3
        sum += it
    }

    for 16 ± 2 |i| {  // 14 ..= 18
        sum += i
    }

    for 7..+3 |i| {   // 7, 8, 9
        sum += i
    }
}
```

`±` can also be written `+-`.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut sum: _ = 0;
    for i in 0..(5) {
        sum += i;
    }
    for it in 1..=(3) {
        sum += it;
    }
    for i in (16 - 2)..=(16 + 2) {
        sum += i;
    }
    for i in 7..(7 + 3) {
        sum += i;
    }
}
```

All four forms lower to Rust's two range types; `±` and `..+` are computed
bounds, not new runtime types.

</details>
