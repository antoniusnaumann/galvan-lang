# while as an Expression

In expression position, a `while` loop collects the result of each iteration
into an array, same as `for` expressions. `continue` skips an
element, so it doubles as a filter:

```galvan
fn main() {
  // TODO: come up with a compelling example
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
  // TODO
}
```

The loop becomes a block expression pushing into a hidden `Vec` — `continue`
naturally skips the push.

</details>
