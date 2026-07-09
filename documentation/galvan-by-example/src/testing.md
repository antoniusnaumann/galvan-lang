# Testing

Tests live next to the code they test — a `test` block in any `.galvan` file
becomes a Rust unit test. The description is optional but encouraged; it
becomes the test's name:

```galvan
test {
    assert 2 == 2
}

test "Addition works correctly" {
    let sum = 2 + 2
    assert sum == 4
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[cfg(test)]
mod tests {

    #[test]
    fn test() {
        {
            assert_eq!(2, 2,);
        };
    }

    #[test]
    fn addition_works_correctly() {
        {
            let sum: _ = 2 + 2;
            assert_eq!(sum, 4,);
        };
    }
}
```

Descriptions are slugified into function names; `assert x == y` becomes
`assert_eq!`, other conditions become `assert!`.

</details>

`assert` checks a condition and fails the test with a helpful message
otherwise. Asserting an equality gets the specialized lowering with both
values in the failure output.

Run tests with plain `cargo test` — they are ordinary Rust tests after
transpilation.
