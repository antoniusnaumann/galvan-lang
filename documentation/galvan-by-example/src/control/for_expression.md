# for as an Expression

In expression position, a `for` loop collects the result of each iteration
into an array — Galvan's answer to list comprehensions. `continue` skips an
element, so it doubles as a filter:

```galvan
fn main() {
    let prices = [1, 4, 5, 7, 10]

    let doubled = for prices |price| {
        price * 2
    }

    let even_doubles: [Int] = for 0..=10 {
        if it % 2 == 1 { continue }
        it * 2
    }

    assert doubled == [2, 8, 10, 14, 20]
    assert even_doubles == [0, 4, 8, 12, 16, 20]
}
```

A `for` expression whose result is expected to be optional auto-wraps, like
any other value (see [Optionals](../errors/optionals.md)).

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let prices: ::std::vec::Vec<_> = vec![1, 4, 5, 7, 10];
    let doubled: ::std::vec::Vec<_> = {
        let mut __result: ::std::vec::Vec<_> = ::std::vec::Vec::new();
        for price in &prices {
            __result.push(price * 2)
        }
        __result
    };
    let even_doubles: ::std::vec::Vec<i64> = {
        let mut __result: ::std::vec::Vec<_> = ::std::vec::Vec::new();
        for it in 0..=(10) {
            if (it % 2).eq(&1) {
                continue;
            };
            __result.push(it * 2)
        }
        __result
    };
    assert_eq!(doubled, vec![2, 8, 10, 14, 20],);
    assert_eq!(even_doubles, vec![0, 4, 8, 12, 16, 20],);
}
```

The loop becomes a block expression pushing into a hidden `Vec` — `continue`
naturally skips the push.

</details>
