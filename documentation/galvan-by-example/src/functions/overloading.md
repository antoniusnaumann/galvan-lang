# Overloading with Labels

Galvan supports a limited, predictable form of overloading: functions with the
same name are distinguished by their **argument labels**. A label is written
before the parameter name; callers must spell it out:

```galvan
fn pick(value: U8) -> U8 {
    value
}

fn pick(value: U8, plus increment: U8) -> U8 {
    value + increment
}

fn pick(value: U8, plus increment: U8, ~ fallback: U8) -> U8 {
    value + increment + fallback
}

fn main() {
    assert pick(1) == 1
    assert pick(2, plus: 3) == 5
    assert pick(4, plus: 5, fallback: 6) == 15
}
```

- `plus increment: U8` declares a parameter named `increment` that callers
  address as `plus:`.
- The `~` marker means "the label is the parameter name itself" —
  `~ fallback` is short for `fallback fallback`.
- The first (unlabeled) parameter never takes a label, which keeps simple
  calls simple.

The same mechanism works for methods:

```galvan
fn adjusted(self: Score) -> U8 { self.value }
fn adjusted(self: Score, by amount: U8) -> U8 { self.value + amount }
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn pick(value: u8) -> u8 {
    value
}

pub(crate) fn pick__plus(value: u8, increment: u8) -> u8 {
    value + increment
}

pub(crate) fn __main__() {
    assert_eq!(pick(1), 1,);
    assert_eq!(pick__plus(2, 3), 5,);
}
```

Overloads become distinct Rust functions whose names are mangled from their
labels (`pick`, `pick__plus`, `pick__plus__fallback`). Galvan identifiers
forbid double underscores precisely so these generated names can never clash
with user code.

</details>
