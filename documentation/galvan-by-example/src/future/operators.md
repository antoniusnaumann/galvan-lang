# Canonical Operators

> [!WARNING]
> **Not implemented yet.** Structural operator derivation does not transpile
> yet. This page describes the intended design.

Galvan's operator model is *structural*: a type supports an operator when all
of its members support it. A vector type gets `+` for free because its
fields are numbers:

```galvan
type Vec2 {
    x: Float
    y: Float
}

test "Automatically derive addition for struct" {
    let this_vec = Vec2(x: 5.0, y: 10.0)
    let that_vec = Vec2(x: 7.0, y: 1.0)

    let result = this_vec + that_vec

    assert result.x == this_vec.x + that_vec.x
    assert result.y == this_vec.y + that_vec.y
}
```

The intended lowering derives `std::ops::Add` (and friends) field-by-field —
the same philosophy as [auto traits](../types/auto_traits.md): capabilities
follow from structure, and boilerplate `impl` blocks are generated, not
written.

Custom operator definitions (including Unicode spellings such as `∧` and `∨`
for domain-specific languages) are planned on top of the same mechanism.
