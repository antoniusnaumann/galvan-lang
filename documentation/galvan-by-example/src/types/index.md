# Custom Types

Every user-defined type in Galvan is declared with the single keyword `type`.
Whether it becomes a Rust struct, tuple struct, enum, or type alias depends
only on the shape of the declaration:

```galvan
type Color { r: Int, g: Int, b: Int }   // struct
type Wrapper(String)                    // tuple struct
type Human = Person                     // alias
type Direction { North South East West } // enum (variants, no fields)
```

Types starting with a lowercase letter are *type parameters* — see
[Generics](../generics/index.md). This chapter walks through each declaration
shape, default field values, and the trait-derivation story.
