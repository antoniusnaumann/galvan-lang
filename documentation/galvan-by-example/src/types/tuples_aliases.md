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
pub(crate) struct Meters(f64);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Segment(Point, Point);
```

</details>

Aliases are also the idiomatic way to name collection shapes:

```galvan
pub type Inventory = {String: Int}
pub type Waypoints = [Point]
```

<details>
<summary>Generated Rust</summary>

```rust
pub type Inventory = ::std::collections::HashMap<String, i64>;

pub type Waypoints = ::std::vec::Vec<Point>;
```

</details>

> [!WARNING]
> **Partially implemented.** Tuple types and their fields *declare*
> correctly, but constructing them positionally (`Meters(1.87)`) does not
> typecheck yet, and tuple member access (`.0`) is still missing. Named tuple
> fields (`type Person(name: String, age: Int)`) do not parse yet either.
