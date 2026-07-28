# Union Types

> [!WARNING]
> **Not implemented yet.** Union types do not parse yet. This page describes
> the intended design.

A union type is written with `|` wherever a type is expected, accepting any
of the listed types:

```galvan
fn print_value(value: Int | String) {
    print("Value: \(value)")
}
```

Unions are Galvan's planned lightweight alternative to declaring a wrapper
enum for "one of these types" situations — particularly at API boundaries
where callers should not need to know a wrapper's name.

In the Rust target, a union parameter is expected to lower to a generated
enum with `From` implementations for each member type, so Rust callers keep a
nameable type while Galvan callers pass members directly.
