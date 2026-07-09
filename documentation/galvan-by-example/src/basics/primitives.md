# Primitive Types

Galvan spells primitive types with an uppercase letter. Integer literals infer
their concrete type from context, like in Rust:

```galvan
fn main() {
    let age: Int = 30
    let byte: U8 = 255
    let big: I128 = 170141183460469231731687303715884105727
    let ratio: Float = 0.5
    let precise: Double = 0.25
    let happy: Bool = true
    let letter: Char = 'g'
    let name: String = "Galvan"
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let age: i64 = 30;
    let byte: u8 = 255;
    let big: i128 = 170141183460469231731687303715884105727;
    let ratio: f32 = 0.5;
    let precise: f64 = 0.25;
    let happy: bool = true;
    let letter: char = 'g';
    let name: String = format!("Galvan");
}
```

</details>

The full mapping to Rust types:

| Galvan | Rust | Galvan | Rust |
| --- | --- | --- | --- |
| `Int` | `i64` | `UInt` | `u64` |
| `I8` … `I128` | `i8` … `i128` | `U8` … `U128` | `u8` … `u128` |
| `ISize` | `isize` | `USize` | `usize` |
| `Float` | `f32` | `Double` | `f64` |
| `Bool` | `bool` | `Char` | `char` |
| `String` | `String` | | |

`Int` and `UInt` are the defaults for everyday code; the sized variants exist
for interop and for data layouts that need them.
