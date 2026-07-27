# while as an Expression

In expression position, a `while` loop collects the result of each iteration
into an array, same as `for` expressions. `continue` skips an
element, so it doubles as a filter:

```galvan
fn main() {
    mut candidate = 0
    let even_squares = while candidate < 6 {
        candidate += 1
        if candidate % 2 == 1 { continue }
        candidate ^ 2
    }

    assert even_squares == [4, 16, 36]
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut candidate: _ = 0;
    let even_squares: ::std::vec::Vec<_> = {
        let mut __result: ::std::vec::Vec<_> = ::std::vec::Vec::new();
        while (candidate).lt(&6) {
            candidate += 1;
            if (candidate % 2).eq(&1) {
                continue;
            };
            __result.push((candidate as i64).pow(2))
        }
        __result
    };
    assert_eq!(even_squares, vec![4, 16, 36],);
}
```

The loop becomes a block expression pushing into a hidden `Vec` — `continue`
naturally skips the push.

</details>
