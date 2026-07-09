# Collection Operators

The collection operators work uniformly across arrays, sets, and strings.
`++` concatenates (or inserts a single element), `++=` does so in place, and
`in` tests membership:

```galvan
fn main() {
    let merged = [1, 2] ++ [5, 6]     // [1, 2, 5, 6]
    let appended = [1, 2] ++ 3        // [1, 2, 3]

    mut stock = {"apples"}
    stock ++= {"bananas", "pears"}    // set union

    let joined = "Hello" ++ " " ++ "World"

    assert 5 in merged
    assert "pears" in stock
}
```

- On arrays, `++` concatenates arrays and appends single elements.
- On sets, `++` is union; `++=` with a single element inserts it.
- On strings, `++` concatenates strings and single characters.
- `in` (also spelled `∈`) works on arrays, sets, and dictionaries (keys).

> [!WARNING]
> **Not implemented yet:** removal `--`, repetition `**`, and slicing `[:]`
> are designed as the counterparts of `++` but do not transpile yet.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let merged: ::std::vec::Vec<_> = [(vec![1, 2]).to_owned(), (vec![5, 6]).to_owned()].concat();
    let appended: ::std::vec::Vec<_> = {
        let mut temp = (vec![1, 2]).to_owned();
        temp.push(3);
        temp
    };
    let mut stock: ::std::collections::HashSet<String> =
        ::std::collections::HashSet::from([format!("apples")]);
    stock = (stock)
        .union(&::std::collections::HashSet::from([
            format!("bananas"),
            format!("pears"),
        ]))
        .cloned()
        .collect::<::std::collections::HashSet<_>>();
    let joined: String = format!(
        "{}{}",
        format!("{}{}", format!("Hello"), format!(" ")),
        format!("World")
    );
    assert!((merged).contains(&(5)));
    assert!((stock).contains(&(format!("pears"))));
}
```

One operator, four lowerings: `concat` for array + array, `push` for array +
element, `union` for sets, and `format!` for strings — the typechecker picks
the right one.

</details>
