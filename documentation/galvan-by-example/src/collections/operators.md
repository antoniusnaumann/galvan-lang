# Collection Operators

The core collection operators work across arrays, sets, and strings. `++`
concatenates (or inserts a single element), `++=` does so in place, and `in`
tests membership. Arrays additionally provide ordered removal. Arrays and
strings support range slicing, and both can be repeated:

```galvan
fn main() {
    let merged = [1, 2] ++ [5, 6]     // [1, 2, 5, 6]
    let appended = [1, 2] ++ 3        // [1, 2, 3]

    mut stock = {"apples"}
    stock ++= {"bananas", "pears"}    // set union

    let joined = "Hello" ++ " " ++ "World"

    let remaining = [1, 2, 3, 1] -- [1, 2, 4]
    let repeated = [1] ** 5
    let letters = 'a' ** 5

    let values = [1, 2, 3, 4, 5]
    let first_two = values[0..=1]
    let near_one = values[1+-1]
    let reversed = values[3..-2]
    let greeting = "Hallöchen"[1..<5]

    assert 5 in merged
    assert "pears" in stock
    assert remaining == [3, 1]
    assert repeated == [1, 1, 1, 1, 1]
    assert letters == "aaaaa"
    assert first_two == [1, 2]
    assert near_one == [1, 2, 3]
    assert reversed == [4, 3, 2]
    assert greeting == "allö"
}
```

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
    let remaining: ::std::vec::Vec<_> = {
        let mut result = (vec![1, 2, 3, 1]).to_owned();
        for removed in (vec![1, 2, 4]).iter() {
            if let Some(index) = result.iter().position(|item| item == removed) {
                result.remove(index);
            }
        }
        result
    };
    let repeated: ::std::vec::Vec<_> = (vec![1]).repeat((5) as usize);
    let letters: String = ('a').to_string().repeat((5) as usize);
    let values: ::std::vec::Vec<_> = vec![1, 2, 3, 4, 5];
    let first_two: ::std::vec::Vec<_> = {
        let __base = &(values);
        (0..=(1))
            .map(|__index| __base[__index as usize].to_owned())
            .collect::<::std::vec::Vec<_>>()
    };
    let near_one: ::std::vec::Vec<_> = {
        let __base = &(values);
        ((1 - 1)..=(1 + 1))
            .map(|__index| __base[__index as usize].to_owned())
            .collect::<::std::vec::Vec<_>>()
    };
    let reversed: ::std::vec::Vec<_> = {
        let __base = &(values);
        (((3 - 2)..=(3)).rev())
            .map(|__index| __base[__index as usize].to_owned())
            .collect::<::std::vec::Vec<_>>()
    };
    let greeting: String = {
        let __chars = (format!("Hallöchen"))
            .chars()
            .collect::<::std::vec::Vec<_>>();
        (1..(5))
            .map(|__index| __chars[__index as usize])
            .collect::<::std::string::String>()
    };
    assert!((merged).contains(&(5)));
    assert!((stock).contains(&(format!("pears"))));
    assert_eq!(remaining, vec![3, 1],);
    assert_eq!(repeated, vec![1, 1, 1, 1, 1],);
    assert_eq!(letters, format!("aaaaa"),);
    assert_eq!(first_two, vec![1, 2],);
    assert_eq!(near_one, vec![1, 2, 3],);
    assert_eq!(reversed, vec![4, 3, 2],);
    assert_eq!(greeting, format!("allö"),);
}
```

One operator, four lowerings: `concat` for array + array, `push` for array +
element, `union` for sets, and `format!` for strings — the typechecker picks
the right one.

</details>

- On arrays, `++` concatenates arrays and appends single elements.
- On sets, `++` is union; `++=` with a single element inserts it.
- On strings, `++` concatenates strings and single characters.
- `in` (also spelled `∈`) works on arrays, sets, and dictionaries (keys).
- Array removal `left -- right` removes the first matching occurrence for
  every element in `right`, ignores missing elements, and preserves the order
  of the remaining values.
- `**` repeats arrays and strings; repeating a character produces a string.
- Indexing an array or string with a range returns an owned value. All range
  forms (`..<`, `..=`, `..+`, `+-`/`±`, and `..-`) work for slicing.
- String indices count Unicode characters rather than UTF-8 bytes.
