# for Loops

`for` iterates over collections, ranges, optionals, results, and iterator
chains. The loop binding goes between pipes after the subject; without pipes,
the implicit binding `it` is available. `break` and `continue` work as usual:

```galvan
fn main() {
    let queue_positions = [1, 2, 3, 4]
    mut total = 0

    for queue_positions |position| {
        if position == 3 { break }
        if position % 2 == 0 { continue }
        total += position
    }

    for queue_positions {
        total += it
    }

    assert total == 11
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let queue_positions: ::std::vec::Vec<_> = vec![1, 2, 3, 4];
    let mut total: _ = 0;
    for position in &queue_positions {
        if (position).eq(&3) {
            break;
        };
        if (position % 2).eq(&0) {
            continue;
        };
        total += position.to_owned();
    }
    for it in &queue_positions {
        total += it.to_owned();
    }
    assert_eq!(total, 11,);
}
```

The loop borrows the collection (`&queue_positions`), so the same array can be
iterated repeatedly — value semantics again.

</details>

- `for subject |binding| { ... }` — explicit binding.
- `for subject { ... }` — implicit `it`.
- Multiple bindings unpack pairs: `for pantry |item, count| { ... }` on a
  dictionary, or `for a.iter().zip(b) |x, y| { ... }`.
- Looping over an optional or a result runs the body zero or one time — see
  [Optionals](../errors/optionals.md).

> [!NOTE]
> Iterating tuples is still incomplete.
