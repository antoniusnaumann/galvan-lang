# Closure Parameters

Function parameters use the same pipe syntax as closure literals: `|T| U` is
the type of a closure from `T` to `U`. Here `floor_map` takes a
float-to-float transformation:

```galvan
fn floor_map(self: [Float], f: |Float| Float) -> [Float] {
    for self {
        f(it.floor())
    }
}

fn main() {
    let measurements = [2.1, 4.5, 6.7]
    let doubled_floors = measurements.floor_map |measurement| { measurement * 2.0 }

    assert doubled_floors == [4.0, 8.0, 12.0]
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub trait Array_Float_Ext {
    fn floor_map(&self, f: &impl Fn(f32) -> f32) -> ::std::vec::Vec<f32>;
}

impl Array_Float_Ext for ::std::vec::Vec<f32> {
    fn floor_map(&self, f: &impl Fn(f32) -> f32) -> ::std::vec::Vec<f32> {
        {
            let mut __result: ::std::vec::Vec<f32> = ::std::vec::Vec::new();
            for &it in self {
                __result.push(f(it.floor()))
            }
            __result
        }
    }
}

pub(crate) fn __main__() {
    let measurements: ::std::vec::Vec<_> = vec![2.1, 4.5, 6.7];
    let doubled_floors: ::std::vec::Vec<f32> =
        (&measurements).floor_map(&(|measurement| measurement * 2.0));
    assert_eq!(doubled_floors, vec![4.0, 8.0, 12.0],);
}
```

Closure parameters lower to `&impl Fn(..)` arguments — static dispatch, no
boxing.

</details>

- `|Float| Float` — one parameter; `|A, B| C` — two; `||` — none.
- Combined with [generics](../generics/functions.md), this builds polymorphic
  helpers like `fn map(self: [t], f: |t| u) -> [u]`.
