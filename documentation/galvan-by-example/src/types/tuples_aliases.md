# Tuple and Alias Types

A `type` with parenthesized, unnamed fields declares a tuple struct — useful
for lightweight wrappers. A `type` with `=` declares an alias. A bare `type`
declares a unit marker type:

```galvan
type Point { x: Double, y: Double }

type Meters(Double)
type Segment(Point, Point)
type Distance = Double
type Marker
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) type Distance = f64;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Marker;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Meters(pub(crate) f64);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Segment(pub(crate) Point, pub(crate) Point);
```

</details>

Aliases are also the idiomatic way to name collection shapes:

```galvan
type Point { x: Double, y: Double }

pub type Inventory = {String: Int}
pub type Waypoints = [Point]
```

<details>
<summary>Generated Rust</summary>

```rust
pub type Inventory = ::std::collections::HashMap<String, i64>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub type Waypoints = ::std::vec::Vec<Point>;
```

</details>

Tuple fields are accessed by their zero-based position:

```galvan
fn main() {
    let position = (1.87, 2.4)
    assert position.0 == 1.87
    assert position.1 == 2.4
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let position: (_, _) = (1.87, 2.4);
    assert_eq!(position.0, 1.87,);
    assert_eq!(position.1, 2.4,);
}
```

</details>

> [!WARNING]
> Named tuple fields (`type Person(name: String, age: Int)`) remain to be
> designed. Positional fields inherit the tuple type's visibility.
